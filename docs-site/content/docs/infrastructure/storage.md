+++
title = "Storage"
description = ""
date = 2024-02-07T08:00:00+00:00
updated = 2024-02-07T08:00:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

In Loco Storage, we facilitate working with files through multiple operations. Storage can be in-memory, on disk, or use cloud services such as AWS S3, GCP, and Azure.

Loco supports simple storage operations and advanced features like mirroring data or backup strategies with different failure modes.

By default, in-memory and disk storage come out of the box. To work with cloud providers, you should specify the following features:
- `storage_aws_s3`
- `storage_azure`
- `storage_gcp`
- `all_storage`

By default loco initialize a `Null` provider, meaning any work with the storage will return an error. 

## Setup

Add the `after_context` function as a Hook in the `app.rs` file and import the `storage` module from `loco_rs`.

```rust
use loco_rs::storage;

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    Ok(ctx)
}
```

This hook returns a Storage instance that holds all storage configurations, covered in the next sections. This Storage instance is stored as part of the application context and is available in controllers, endpoints, task workers, and more.

## Glossary
|          |   |
| -        | - |
| `StorageDriver` | Trait implementation something that does storage  |
| `Storage`| Abstraction implementation for managing one or more storage drivers. |
| `Strategy`| Trait implementing various strategies for Storage, such as mirror or backup. |
| `FailureMode`| Implemented within each Strategy, determining how to handle operations in case of failures. |

### Initialize Storage

Storage can be configured with a single driver or multiple drivers.

#### Single Store

In this example, we initialize the in-memory driver and create a new storage with the single function.

```rust
use loco_rs::storage;

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    Ok(AppContext {
        storage: Storage::single(storage::drivers::mem::new()).into(),
        ..ctx
    })
}
```

### Multiple Drivers

For advanced usage, you can set up multiple drivers and apply smart strategies that come out of the box. Each strategy has its own set of failure modes that you can decide how to handle.

Creating multiple drivers:

```rust
use crate::storage::{drivers, Storage};

let aws_1 = drivers::aws::new("users");
let azure = drivers::azure::new("users");
let aws_2 = drivers::aws::new("users-mirror");
```

#### Mirror Strategy:
You can keep multiple services in sync by defining a mirror service. A mirror service **replicates** uploads, deletes, rename and copy across two or more subordinate services. The download behavior redundantly retrieves data, meaning if the file retrieval fails from the primary, the first file found in the secondaries is returned.

#### Behaviour

After creating the three store instances, we need to create the mirror strategy instance and define the failure mode. The mirror strategy expects the primary store and a list of secondary stores, along with failure mode options:
- `MirrorAll`: All secondary storages must succeed. If one fails, the operation continues to the rest but returns an error.
- `AllowMirrorFailure`: The operation does not return an error when one or more mirror operations fail.

The failure mode is relevant for upload, delete, move, and copy.

Example:
```rust

// Define the mirror strategy by setting the primary store and secondary stores by names.
let strategy = Box::new(MirrorStrategy::new(
    "store_1",
    Some(vec!["store_2".to_string(), "store_3".to_string()]),
    FailureMode::MirrorAll,
)) as Box<dyn StorageStrategy>;

// Create the storage with the store mapping and the strategy.
 let storage = Storage::new(
    BTreeMap::from([
        ("store_1".to_string(), aws_1),
        ("store_2".to_string(), azure),
        ("store_3".to_string(), aws_2),
    ]),
    strategy.into(),
);
```

### Backup Strategy:

You can back up your operations across multiple storages and control the failure mode policy.

After creating the three store instances, we need to create the backup strategy instance and define the failure mode. The backup strategy expects the primary store and a list of secondary stores, along with failure mode options:
- `BackupAll`: All secondary storages must succeed. If one fails, the operation continues to the rest but returns an error.
- `AllowBackupFailure`: The operation does not return an error when one or more backup operations fail.
- `AtLeastOneFailure`: At least one operation should pass.
- `CountFailure`: The given number of backups should pass.

The failure mode is relevant for upload, delete, move, and copy. The download always retrieves the file from the primary.

Example:
```rust

// Define the backup strategy by setting the primary store and secondary stores by names.
let strategy: Box<dyn StorageStrategy> = Box::new(BackupStrategy::new(
    "store_1",
    Some(vec!["store_2".to_string(), "store_3".to_string()]),
    FailureMode::AllowBackupFailure,
)) as Box<dyn StorageStrategy>;

let storage = Storage::new(
    BTreeMap::from([
        ("store_1".to_string(), store_1),
        ("store_2".to_string(), store_2),
        ("store_3".to_string(), store_3),
    ]),
    strategy.into(),
);
```

## Create Your Own Strategy

In case you have a specific strategy, you can easily create it by implementing the StorageStrategy and implementing all store functionality.

## Usage In Controller

Follow this example, make sure you enable `multipart` feature in axum crate.

```rust
use loco_rs::prelude::*;

async fn upload_file(
    State(ctx): State<AppContext>,
    mut multipart: Multipart,
) -> Result<Response> {
    let mut file = None;
    while let Some(field) = multipart.next_field().await.map_err(|err| {
        tracing::error!(error = ?err,"could not readd multipart");
        Error::BadRequest("could not readd multipart".into())
    })? {
        let file_name = match field.file_name() {
            Some(file_name) => file_name.to_string(),
            _ => return Err(Error::BadRequest("file name not found".into())),
        };

        let content = field.bytes().await.map_err(|err| {
            tracing::error!(error = ?err,"could not readd bytes");
            Error::BadRequest("could not readd bytes".into())
        })?;

        let path = PathBuf::from("folder").join(file_name);
        ctx.storage.as_ref().upload(path.as_path(), &content).await?;

        file = Some(path);
    }

    file.map_or_else(not_found, |path| {
        format::json(views::upload::Response::new(path.as_path()))
    })
}
```
# Testing

By testing file storage in your controller you can follow this example:

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_register() {
    request::<App, _, _>(|request, ctx| async move {
        let file_content = "loco file upload";
        let file_part = Part::bytes(file_content.as_bytes()).file_name("loco.txt");

        let multipart_form = MultipartForm::new().add_part("file", file_part);

        let response = request.post("/upload/file").multipart(multipart_form).await;

        response.assert_status_ok();

        let res: views::upload::Response = serde_json::from_str(&response.text()).unwrap();

        let stored_file: String = ctx.storage.as_ref().download(&res.path).await.unwrap();

        assert_eq!(stored_file, file_content);
    })
    .await;
}
```

