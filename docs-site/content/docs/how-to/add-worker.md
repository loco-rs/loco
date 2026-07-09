+++
title = "Add a background worker"
description = "Generate a worker, implement BackgroundWorker, enqueue a job with perform_later, and register it in connect_workers."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/processing/workers/"]

[extra]
lead = ""
toc = true
top = false
+++

Goal: move slow or non-request-critical work (sending a report, calling a third-party API, resizing an image) out of the request path and into a background job.

## Prerequisites

- A queue backend configured (Redis, Postgres, or SQLite) if you want jobs to survive a restart. If you haven't decided yet, see [Choose a queue backend](@/docs/how-to/choose-queue-backend.md). For local dev you can skip this — the default `BackgroundQueue` mode with no `queue:` config still works, it just won't persist jobs (jobs are dropped with a logged error if no provider is populated). Many apps start with `workers.mode: BackgroundAsync`, which needs no queue backend at all.

## 1. Generate the worker

```sh
cargo loco generate worker report_worker
```

This creates `src/workers/report_worker.rs`, adds `pub mod report_worker;` to `src/workers/mod.rs`, and injects a registration call into `connect_workers` in `src/app.rs`. It also generates a test stub under `tests/workers/`.

The generated struct is always named `Worker` (scoped inside its own `workers::report_worker` module), with an empty `WorkerArgs` struct for you to fill in:

```rust
use serde::{Deserialize, Serialize};
use loco_rs::prelude::*;

pub struct Worker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct WorkerArgs {}

#[async_trait]
impl BackgroundWorker<WorkerArgs> for Worker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "ReportWorker".to_string()
    }

    async fn perform(&self, _args: WorkerArgs) -> Result<()> {
        // TODO: your job logic goes here
        Ok(())
    }
}
```

## 2. Add typed arguments and job logic

Fill in `WorkerArgs` with whatever data the job needs (it's serialized into the queue, so keep it small and `Serialize + Deserialize`), then implement `perform`:

```rust
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

pub struct DownloadWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DownloadWorkerArgs {
    pub user_guid: String,
}

#[async_trait]
impl BackgroundWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: DownloadWorkerArgs) -> Result<()> {
        // .. do the actual work, use self.ctx for DB/cache/etc ..
        println!("processing download for {}", args.user_guid);
        Ok(())
    }
}
```

(This example mirrors `examples/demo/src/workers/downloader.rs`.)

## 3. Confirm it's registered

The generator already injected this, but it's worth knowing what it did — `Hooks::connect_workers` is where every worker is registered against the shared `Queue`:

```rust
// src/app.rs
#[async_trait]
impl Hooks for App {
    // ..
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }
    // ..
}
```

If you wrote the worker manually instead of generating it, add the `queue.register(...)` line yourself.

## 4. Enqueue a job

Call the trait's `perform_later` from a controller, task, or another worker:

```rust
DownloadWorker::perform_later(
    &ctx,
    DownloadWorkerArgs {
        user_guid: "foo".to_string(),
    },
)
.await?;
```

`perform_later` returns `Result<String>` — the job id, not `Result<()>`. In `BackgroundQueue` mode the id is assigned by the queue provider; in `ForegroundBlocking`/`BackgroundAsync` mode (or when no provider is configured) Loco generates a fresh UUID so you always get a stable handle back:

```rust
let job_id: String = DownloadWorker::perform_later(&ctx, args).await?;
```

If you need higher/lower priority for this particular job, use `perform_later_with_priority` instead — see [Choose a queue backend](@/docs/how-to/choose-queue-backend.md#priority-queues) for priority semantics shared across all three backends:

```rust
DownloadWorker::perform_later_with_priority(&ctx, args, Some(50)).await?;
```

## 5. Run the worker process

How you run workers depends on `workers.mode` (see [Choose a queue backend](@/docs/how-to/choose-queue-backend.md)):

```sh
# BackgroundQueue mode: run a dedicated worker process
cargo loco start --worker

# or run server + worker in the same process
cargo loco start --server-and-worker
```

`BackgroundAsync` and `ForegroundBlocking` modes don't need a separate worker process — jobs run inside whichever process called `perform_later`.

### Filtering by tags

Give a worker tags, then start a worker process that only picks up matching jobs:

```rust
fn tags() -> Vec<String> {
    vec!["download".to_string(), "network".to_string()]
}
```

```sh
cargo loco start --worker download,network
```

A worker started with no tags (`cargo loco start --worker`) only processes untagged jobs; `--all` and `--server-and-worker` don't support tag filtering.

## 6. Verify

Test with `ForegroundBlocking` mode set in `config/test.yaml`, so `perform_later` runs synchronously and returns only once the job is done:

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn test_run_download_worker() {
    let boot = boot_test::<App, Migrator>().await.unwrap();

    assert!(
        DownloadWorker::perform_later(
            &boot.app_context,
            DownloadWorkerArgs { user_guid: "foo".to_string() }
        )
        .await
        .is_ok()
    );

    // .. assert side effects here ..
}
```

Put worker tests under `tests/workers/` — the generator does this for you automatically.

## Reference

- Every `queue:`/`workers:` YAML key: [Configuration reference](@/docs/reference/configuration.md#queue)
- `cargo loco start`/`jobs` flags: [CLI reference](@/docs/reference/cli.md)
- `worker`/`worker_redis` feature flags: [Feature flags reference](@/docs/reference/feature-flags.md)
