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


## Overview 🌟

In Loco Storage, we enable seamless file management through a variety of operations. Whether you are handling data in-memory, on local disk, or via cloud services such as AWS S3, GCP, or Azure, Loco provides a flexible and robust solution.

Loco supports essential storage tasks such as uploading, downloading, deleting, renaming, copying, and checking existence. For advanced scenarios, it offers features like data mirroring or backup strategies with customizable failure modes to ensure reliability.

By default, in-memory and disk storage are available out-of-the-box. To integrate cloud providers, enable the relevant Cargo features:

- `storage_aws_s3` for AWS S3
- `minio` for MinIO
- `storage_azure` for Azure Blob Storage
- `storage_gcp` for Google Cloud Storage
- `all_storage` to include all cloud options

Without configuration, Loco initializes a `Null` provider, which returns errors for any storage operations—ensuring you set up a proper driver before use.

## Setup

To integrate storage into your Loco application, add an `after_context` hook in your `app.rs` file. Import the `storage` module from `loco_rs` and configure your driver(s).

Here's a basic example using the in-memory driver:

```rust
use loco_rs::storage;

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    let mem_driver = storage::drivers::mem::new();

    Ok(AppContext {
        storage: Storage::single(mem_driver).into(),
        ..ctx
    })
}
```

This hook injects a `Storage` instance into the application context, making it accessible in controllers, endpoints, tasks, and more.

## Glossary 📚

Here's a quick reference for key terms:

| Term             | Description                                                                 |
|------------------|-----------------------------------------------------------------------------|
| **StorageDriver** | A trait implementation for actual storage backends (e.g., AWS, local disk). |
| **Storage**      | An abstraction layer managing one or more drivers with strategies.          |
| **Strategy**     | A trait for advanced behaviors like mirroring or backups.                   |
| **FailureMode**  | Defines how strategies handle errors during operations.                     |

## Initializing Storage 🛠️

Storage can be set up with a single driver for simplicity or multiple drivers for redundancy and advanced strategies.

### Single Store

For basic use, initialize one driver and wrap it in `Storage::single`:

```rust
   Storage::single(storage::drivers::mem::new()).into(),
```

### Multiple Drivers

For scenarios requiring redundancy, create multiple drivers and apply strategies like mirroring or backups.

First, define your drivers:

```rust
use loco_rs::storage::drivers;

let aws_1 = drivers::aws::new("users");
let azure = drivers::azure::new("users");
let aws_2 = drivers::aws::new("users-mirror");
```

#### Mirror Strategy

The mirror strategy replicates operations (upload, delete, rename, copy) across a primary and secondary stores, keeping them in sync. For downloads, it falls back to secondaries if the primary fails.

**Failure Modes**:

- `MirrorAll`: All mirrors must succeed; errors from any cause the operation to fail.
- `AllowMirrorFailure`: Continues even if mirrors fail, without returning an error.

Example configuration:

```rust
use loco_rs::storage::{MirrorStrategy, FailureMode, Storage, StorageStrategy};
use std::collections::BTreeMap;

let strategy: Box<dyn StorageStrategy> = Box::new(MirrorStrategy::new(
    "store_1",
    Some(vec!["store_2".to_string(), "store_3".to_string()]),
    FailureMode::MirrorAll,
));

let storage = Storage::new(
    BTreeMap::from([
        ("store_1".to_string(), aws_1),
        ("store_2".to_string(), azure),
        ("store_3".to_string(), aws_2),
    ]),
    strategy.into(),
);
```

#### Backup Strategy

The backup strategy performs operations on the primary and replicates them to backups. Downloads always use the primary.

**Failure Modes**:

- `BackupAll`: All backups must succeed; errors cause failure.
- `AllowBackupFailure`: Ignores backup failures.
- `AtLeastOneSuccess`: Ensures at least one backup succeeds.
- `CountSuccess`: Requires a specified number of backups to succeed.

Example:

```rust
use loco_rs::storage::{BackupStrategy, FailureMode, Storage, StorageStrategy};
use std::collections::BTreeMap;

let strategy: Box<dyn StorageStrategy> = Box::new(BackupStrategy::new(
    "store_1",
    Some(vec!["store_2".to_string(), "store_3".to_string()]),
    FailureMode::AllowBackupFailure,
));

let storage = Storage::new(
    BTreeMap::from([
        ("store_1".to_string(), aws_1),
        ("store_2".to_string(), azure),
        ("store_3".to_string(), aws_2),
    ]),
    strategy.into(),
);
```

## Creating Your Own Strategy 🛠️✨

If built-in strategies do not fit, implement the `StorageStrategy` trait to define custom logic for all operations (upload, get, delete, etc.).

## Usage in Controllers 📝

Enable the `multipart` feature in Axum for file uploads. Here's an example controller for uploading files:

```rust
use axum::{extract::{Multipart, State}, response::Response};
use loco_rs::{prelude::*, storage::Storage};
use std::path::PathBuf;

async fn upload_file(
    State(ctx): State<AppContext>,
    mut multipart: Multipart,
) -> Result<Response> {
    let mut file = None;
    while let Some(field) = multipart.next_field().await.map_err(|err| {
        tracing::error!(error = ?err, "could not read multipart");
        Error::BadRequest("could not read multipart".into())
    })? {
        let file_name = field.file_name().ok_or(Error::BadRequest("file name not found".into()))?.to_string();

        let content = field.bytes().await.map_err(|err| {
            tracing::error!(error = ?err, "could not read bytes");
            Error::BadRequest("could not read bytes".into())
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

## Testing 🧪

Test storage interactions in controllers using Loco's testing utilities:

```rust
use loco_rs::testing::prelude::*;
use axum::multipart::{MultipartForm, Part};

#[tokio::test]
#[serial]
async fn can_upload() {
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

## Quckly start (Minio/S3 Example)

Loco makes file storage a breeze, whether you're saving files locally, in-memory, or on cloud platforms like AWS S3 or MinIO. This guide walks you through setting up storage, configuring cloud providers, and creating endpoints to upload and retrieve files. Let's dive in! 🌊

### 1. Enable Storage Features 📚

To use cloud storage, enable the necessary features in your `Cargo.toml` file. This tells Loco which storage backends you want to support.
Add the necessary features to your `Cargo.toml`:

```toml
loco-rs = { version = "newest-version", features = [ "minio"] }
```

**Note**: Replace `"newest-version"` with the latest `loco-rs` version. If you're only using local or in-memory storage, you can skip the cloud features.

### 2. Configure Your Storage Backend 🛠️

#### 2.1 Minio Setup ☁️

Set up your storage in the `src/app.rs` file by adding a driver to the `after_context` hook. This makes storage available across your app (controllers, tasks, etc.).

```rust
// src/app.rs

use loco_rs::storage::{aws, drivers::aws::Credential};

async fn after_context(ctx: AppContext) -> Result<AppContext> {
        let credential = minio::Credential {
            key_id: "...".to_string(),
            secret_key: "...".to_string(),
            endpoint: "http://127.0.1:9000".to_string(),
        };

        let driver = minio::with_bucket_and_credentials("bucket-name",credential);

        Ok(AppContext {
            storage: Storage::single(driver.unwrap()).into(),
            ..ctx
        })
}
```

**Tips**:

- Update the `endpoint` to match your MinIO server (e.g., `http://localhost:9000`).
- Set `MINIO_ACCESS_KEY` and `MINIO_SECRET_KEY` in your environment variables.

#### 2.2 AWS S3 Setup ☁️

For AWS S3, provide your bucket name, region, and credentials.

```rust
// src/app.rs
use loco_rs::storage::{aws, drivers::aws::Credential};

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    let credential = Credential {
        key_id: "AWS_ACCESS_KEY_ID",
        secret_key: "AWS_ACCESS_KEY_SECRET",
        token: None,
    };

    let driver = aws::with_credentials("my-bucket", "ap-south-1", credential);

    Ok(AppContext {
        storage: Storage::single(driver.unwrap()).into(),
        ..ctx
    })
}
```

**Tips**:

- Store `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` in environment variables for security.
- Use `aws::with_credentials_and_endpoint` if you need a custom S3 endpoint (e.g., for S3-compatible services).

### 3. Create Storage Endpoints 🎮

Add controllers to handle file uploads and downloads. We'll create two endpoints: one to upload files and another to retrieve them, mimicking Rails' Active Storage format for compatibility.

#### 3.1 Upload Function

```rust
use axum::{extract::{Multipart, State}, response::Response};
use loco_rs::{prelude::*, storage::Storage};
use std::path::PathBuf;

#[debug_handler]
async fn upload_file(State(ctx): State<AppContext>, mut multipart: Multipart) -> Result<Response> {
    while let Some(field) = multipart.next_field().await.map_err(|err| {
        tracing::error!(error = ?err, "could not read multipart");
        Error::BadRequest("could not read multipart".into())
    })? {
        let file_name = match field.file_name() {
            Some(file_name) => file_name.to_string(),
            _ => return Err(Error::BadRequest("file name not found".into())),
        };

        let content = field.bytes().await.map_err(|err| {
            tracing::error!(error = ?err, "could not read bytes");
            Error::BadRequest("could not read bytes".into())
        })?;

        // Construct S3-compatible key (virtual path)
        let pid = uuid::Uuid::new_v4(); // you should change to signed_id if you want to follow active storage process
        let key = format!("uploads/{}", pid);
        let path = PathBuf::from(&key);

        let _res = ctx
            .storage
            .as_ref()
            .upload(path.as_path(), &content)
            .await?;

        return format::json(data!({
        "message": "File uploaded successfully",
          "file": {
                "pid": pid.to_string(), 
                "filename": file_name,
                "byte_size": content.len(),
                // You can also save in database or as activestorage data structure
            
          }
        }));
    }

    bad_request("No files were uploaded")
}


pub fn routes() -> Routes {
    Routes::new()
        .prefix("/storage")
        .add("/upload", post(upload_file))
}

```

## 4. Test Your Setup 🧪

Try uploading and downloading a file to ensure everything works.

### Upload a File

```bash
curl -X POST http://localhost:5150/storage/upload \
  -F "file=@example.txt" \
  -H "Content-Type: multipart/form-data"
```

**Expected Response**:

```json
{
  "message": "File uploaded successfully",
  "file": {
    "pid": "550e8400-e29b-41d4-a716-446655440000",
    "filename": "example.txt",
    "byte_size": 123,
    //...etc
  }
}
```

### Download a File

```rust
use axum::{extract::{Multipart, State}, response::Response};
use loco_rs::{prelude::*, storage::Storage};
use std::path::PathBuf;


async fn show_file(State(ctx): State<AppContext>, pid: String) -> Result<Response> {
    let key = format!("uploads/{}", pid);
    let path = PathBuf::from(&key);
    let content: Vec<u8> = ctx.storage.as_ref().download(&path).await?;

    if content.is_empty() {
        return not_found();
    }

    Ok(Response::builder()
        .header("Content-Type", "application/octet-stream")
        .body(content.into())
        .unwrap())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("storage/")
        .add("/{pid}", get(show_file))
}
```ç

Use the `pid` from the upload response:
(Note: pid just for demo purpose)

```bash
curl http://localhost:3000/storage/eyJhbGciOiJIUzI1NiJ9...
```

**Expected**: The file content is returned with appropriate headers, or a `404` if the file is missing.

## 5. Tips for Success 🌟

- **Secure Your Secrets**: Store `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `MINIO_ACCESS_KEY`, `MINIO_SECRET_KEY`, and `JWT_SECRET` in environment variables or a secret manager.
- **File Metadata**: Save file details (e.g., `filename`, `content_type`) in a database during upload to improve `show_file`’s content type accuracy.
- **Validation**: Add file size and extension checks to `upload_file` for security (e.g., limit to 10 MB, allow only `.txt`, `.pdf`).
- **Production**: Add authentication and rate limiting to protect your endpoints.

Happy file storing with Loco! 🚀 If you hit any snags, check your logs or reach out for help.
