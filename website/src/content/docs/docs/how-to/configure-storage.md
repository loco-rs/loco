---
title: Configure file storage
description: Wire up the Storage API over local disk, in-memory, or cloud (S3/Azure/GCS) drivers, pick a mirror/backup strategy, and stream large files.
sidebar:
  order: 30
---

Goal: give your app a place to put uploaded files — on disk, in memory (for tests), or in a cloud bucket — through one consistent `Storage` API, without hand-rolling an OpenDAL client yourself.

Loco's storage layer is a thin abstraction over [Apache OpenDAL](https://opendal.apache.org/). Every driver ends up implementing the same `StoreDriver` trait, so your controller code doesn't change when you swap local disk for S3.

## Prerequisites

Local, in-memory, and null storage work with no extra Cargo features. Cloud drivers need one of:

```toml
loco-rs = { version = "...", features = ["storage_aws_s3"] } # or storage_azure, storage_gcp, all_storage
```

See the [feature flags reference](@/docs/reference/feature-flags.md) for the full matrix.

## 1. Wire up a single driver

Storage isn't configured in YAML — it's wired in code, in the `after_context` hook (`src/app.rs`), and lands on `ctx.storage: Arc<Storage>`.

```rust
use loco_rs::storage::{self, drivers};

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    Ok(AppContext {
        storage: storage::Storage::single(drivers::local::new()).into(),
        ..ctx
    })
}
```

If you don't override `after_context` at all, Loco defaults to the **`Null` driver** — every storage operation returns `StorageError::Any("Operation not supported by null storage")`. That's a deliberate fail-fast default, not a bug: it means "you haven't wired storage yet."

## 2. Pick a driver

Every driver is built by a plain constructor function under `loco_rs::storage::drivers::*` — no trait object wrangling required.

| Driver | Feature | Constructor | Notes |
|---|---|---|---|
| Local filesystem | none | `drivers::local::new()` — rooted at the current working directory<br>`drivers::local::new_with_prefix(prefix) -> StorageResult<Box<dyn StoreDriver>>` | `new_with_prefix` errors if the prefix path doesn't exist |
| In-memory | none | `drivers::mem::new()` | Good for tests; data doesn't survive process exit |
| Null | none | `drivers::null::new()` | The framework default; every op errors |
| AWS S3 | `storage_aws_s3` | `drivers::aws::new(bucket, region) -> StorageResult<...>`<br>`drivers::aws::with_credentials(bucket, region, cred) -> StorageResult<...>`<br>`drivers::aws::with_credentials_and_endpoint(bucket, region, endpoint, cred) -> StorageResult<...>` | `Credential { key_id, secret_key, token: Option<String> }` |
| Azure Blob | `storage_azure` | `drivers::azure::new(container, account_name, access_key, endpoint) -> StorageResult<...>` | |
| Google Cloud Storage | `storage_gcp` | `drivers::gcp::new(bucket, credential_path) -> StorageResult<...>` | `credential_path` points to a service-account JSON key file |

All the cloud constructors return `StorageResult<Box<dyn StoreDriver>>` (they can fail to build the underlying OpenDAL operator), so propagate the error with `?`:

```rust
use loco_rs::storage::{self, drivers};

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    let store = drivers::aws::new("my-app-uploads", "us-east-1")?;
    Ok(AppContext {
        storage: storage::Storage::single(store).into(),
        ..ctx
    })
}
```

For credentials that aren't in the environment/instance profile, pass them explicitly:

```rust
use loco_rs::storage::drivers::aws::{self, Credential};

let credential = Credential {
    key_id: std::env::var("AWS_ACCESS_KEY_ID")?,
    secret_key: std::env::var("AWS_SECRET_ACCESS_KEY")?,
    token: None,
};
let store = aws::with_credentials("my-app-uploads", "us-east-1", credential)?;
```

> The storage driver trait is `StoreDriver` (not `StorageDriver`) — you'll see it in error messages and if you implement your own driver.

## 3. Use multiple drivers with a strategy (optional)

For redundancy across providers, set up several named stores and a `StorageStrategy` that decides how operations fan out across them.

**Mirror** — replicates uploads/deletes/renames/copies to every store; download tries the primary, then falls through to secondaries on failure. This is `ReplicatedStrategy::mirror`.

```rust
use std::collections::BTreeMap;
use loco_rs::storage::{
    drivers, Storage,
    strategies::{replicated::{ReplicatedStrategy, FailurePolicy}, StorageStrategy},
};

let primary = drivers::aws::new("bucket-primary", "us-east-1")?;
let mirror = drivers::azure::new("container", "account", "access-key", "https://account.blob.core.windows.net")?;

let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
    "primary",
    Some(vec!["mirror".to_string()]),
    FailurePolicy::FailIfAny, // or AllowAll
));

let storage = Storage::new(
    BTreeMap::from([
        ("primary".to_string(), primary),
        ("mirror".to_string(), mirror),
    ]),
    strategy,
);
```

`FailurePolicy::FailIfAny` requires every secondary to succeed (errors bubble up as `StorageError::Multi`); `AllowAll` swallows secondary failures.

**Backup** — the primary must always succeed for writes; secondary failures are governed by a separate failure policy, and downloads *always* come from the primary only. This is `ReplicatedStrategy::backup`.

```rust
use loco_rs::storage::strategies::replicated::{ReplicatedStrategy, FailurePolicy};

let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
    "primary",
    Some(vec!["backup_store".to_string()]),
    FailurePolicy::AllowAll, // also: FailIfAny, AllowSingleFailure, FailAtFailures(n)
));
```

Mirror and backup are both `ReplicatedStrategy`, differing only in the constructor used (`mirror` vs `backup`) and the `FailurePolicy` you pick. It exposes a `_with_policy`/`_with_strategy` variant on every `Storage` method (`upload_with_strategy`, `download_with_policy`, ...) if you need to override the strategy for a single call.

## 4. Upload and download in a controller

```rust
use loco_rs::prelude::*;
use std::path::PathBuf;

async fn upload_file(
    State(ctx): State<AppContext>,
    mut multipart: Multipart,
) -> Result<Response> {
    while let Some(field) = multipart.next_field().await.map_err(|_| {
        Error::BadRequest("could not read multipart".into())
    })? {
        let file_name = field
            .file_name()
            .map(str::to_string)
            .ok_or_else(|| Error::BadRequest("file name not found".into()))?;

        let content = field
            .bytes()
            .await
            .map_err(|_| Error::BadRequest("could not read bytes".into()))?;

        let path = PathBuf::from("uploads").join(file_name);
        ctx.storage.as_ref().upload(&path, &content).await?;

        return format::json(serde_json::json!({ "path": path }));
    }
    not_found()
}
```

(Requires the `multipart` feature on the `axum` crate.)

## 5. Stream large files instead of buffering them

For files too large to comfortably hold in memory, use the streaming API — `download_stream`/`upload_stream` return/accept a `BytesStream`, which converts directly to/from an axum `Body`. This is undocumented in earlier Loco releases but is a stable, full public feature.

Streaming a download straight into an HTTP response, with zero extra buffering:

```rust
use axum::response::IntoResponse;
use std::path::Path;

async fn download_video(State(ctx): State<AppContext>) -> Result<impl IntoResponse> {
    let stream = ctx.storage.download_stream(Path::new("videos/demo.mp4")).await?;
    Ok(stream.into_body())
}
```

Streaming an upload from an incoming request body (axum's `Body` stream yields `axum::Error`, so map it to `std::io::Error` first — that's the error type `BytesStream` expects):

```rust
use loco_rs::storage::stream::BytesStream;
use futures_util::StreamExt;
use std::path::Path;

async fn upload_video(State(ctx): State<AppContext>, body: axum::body::Body) -> Result<Response> {
    let mapped = body
        .into_data_stream()
        .map(|chunk| chunk.map_err(std::io::Error::other));
    let stream = BytesStream::from_body_stream(mapped);
    ctx.storage.upload_stream(Path::new("videos/demo.mp4"), stream).await?;
    format::empty()
}
```

If you need the whole payload as one `Bytes` buffer anyway, `BytesStream::collect()` gives you that — but at that point you've given up the memory benefit of streaming.

**Strategy caveat:** streaming isn't uniformly "true streaming" once a strategy other than `SingleStrategy` is involved. `ReplicatedStrategy` unifies the former mirror/backup behavior: reads (both the buffered `download` and `download_stream`) fall back to secondaries when `read_from_secondaries` is set — i.e. constructed via `ReplicatedStrategy::mirror` — and are served from the primary only when constructed via `ReplicatedStrategy::backup`. Either way, `upload_stream` buffers the whole payload once via `collect()` and then fans out concurrently to secondaries. If you need guaranteed zero-buffering streaming to a single store, stick to `SingleStrategy` (the default).

## 6. Verify

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_upload_and_download() {
    request::<App, _, _>(|request, ctx| async move {
        let file_content = "loco file upload";
        let file_part = Part::bytes(file_content.as_bytes()).file_name("loco.txt");
        let multipart_form = MultipartForm::new().add_part("file", file_part);

        let response = request.post("/upload/file").multipart(multipart_form).await;
        response.assert_status_ok();

        let res: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let path = res["path"].as_str().unwrap();

        let stored: String = ctx.storage.as_ref().download(&std::path::Path::new(path)).await.unwrap();
        assert_eq!(stored, file_content);
    })
    .await;
}
```

## Reference

- `storage_aws_s3` / `storage_azure` / `storage_gcp` / `all_storage` feature flags: [Feature flags reference](@/docs/reference/feature-flags.md)
- Storage has no YAML configuration surface — everything above is the complete configuration story; there is no `storage:` key to look up in the [Configuration reference](@/docs/reference/configuration.md)
