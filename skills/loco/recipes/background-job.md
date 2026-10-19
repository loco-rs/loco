# Recipe: background jobs

Loco's `BackgroundWorker` is Rails' Active Job. Use it when an HTTP request must
return before the work finishes: sending email, generating thumbnails, calling a
slow third party, fanning out webhooks.

**`tokio::spawn` is never the alternative.** A spawned future is not durable,
not retried, not observable, and dies with the process. If you are reaching for
it, you want a worker.

```sh
cargo loco generate worker onboarding
```

## The worker

```rust
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::users;

pub struct OnboardingWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct OnboardingWorkerArgs {
    pub user_id: i32,
}

#[async_trait]
impl BackgroundWorker<OnboardingWorkerArgs> for OnboardingWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: OnboardingWorkerArgs) -> Result<()> {
        let user = users::Model::find_by_id(&self.ctx.db, args.user_id).await?;
        tracing::info!(user_id = user.id, "running onboarding");
        Ok(())
    }
}
```

Args must be `Serialize + Deserialize` — they are persisted to the queue, so
pass **identifiers, not objects**. Sending a whole `Model` means the worker acts
on a stale snapshot; send `user_id` and re-load.

The generator registers it in `src/app.rs`:

```rust
async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
    queue.register(workers::onboarding::OnboardingWorker::build(ctx)).await?;
    Ok(())
}
```

## Enqueue it

```rust
OnboardingWorker::perform_later(&ctx, OnboardingWorkerArgs { user_id: user.id }).await?;
```

`perform_later` returns `Result<String>` — the job id. There is also
`perform_later_with_priority(ctx, args, Some(priority))`.

To fan out many jobs, enqueue them in one call rather than looping:

```rust
// one round trip, one transaction, atomic on every backend; returns one id
// per job, in input order
let job_ids = OnboardingWorker::perform_all_later(&ctx, args_list).await?;

// same, with a priority per job (None = default)
OnboardingWorker::perform_all_later_with_priority(
    &ctx,
    vec![(a, Some(100)), (b, None)],
)
.await?;
```

Calling `perform_later` in a loop to enqueue N jobs costs N round trips and
leaves the queue half-populated if one of them fails.

Optional trait methods:

```rust
fn queue() -> Option<String> { Some("critical".to_string()) }
fn tags() -> Vec<String> { vec!["email".to_string()] }
```

## Unique jobs

To collapse repeat enqueues of the same work (a webhook storm, a "recompute X"
trigger), enqueue with a key:

```rust no-syntax-check="a call statement, not an item"
// perform_later_unique(ctx: &AppContext, args: A, key: String) -> Result<String>
OnboardingWorker::perform_later_unique(
    &ctx,
    OnboardingWorkerArgs { user_id: user.id },
    format!("onboarding:{}", user.id),
)
.await?;
```

While a job of the same worker with the same key is still **queued**, a repeat
call adds nothing and returns that job's id. Once it starts running, a new one
can queue, so work that arrives mid-run is not lost. A unique index enforces
this, so concurrent callers share one job. Postgres and SQLite queues only:
Redis returns an error. In `ForegroundBlocking` / `BackgroundAsync` modes it
runs like `perform_later`. The lower-level form is
`queue.enqueue_unique(class, queue, args, tags, priority, key)`.

Retries keep the rule. When a unique job fails, or the reaper picks it up,
while a twin with the same key is already queued, it is cancelled rather than
requeued, because the twin does the same work. `retry_failed` leaves such a job
`failed`.

## Retries: return `Err`, set the policy on the worker

A loop with `tokio::time::sleep` and an attempt counter adds no dependency and
is still wrong — it blocks a worker slot, loses state on restart, and silently
swallows the failure when the counter runs out. **Return `Err` and let the queue
retry.** The retry policy is two optional trait methods on the worker:

```rust no-syntax-check="trait methods shown outside their impl block"
fn max_retries() -> u32 { 5 }              // default 0: the first failure is final
fn retry_backoff() -> std::time::Duration { // default 5s; doubles per retry, capped at 1h
    std::time::Duration::from_secs(10)
}
```

Postgres and SQLite queues only, in `BackgroundQueue` mode. Registering a worker
with `max_retries() > 0` on Redis is an error.

## Long jobs and the reaper

With a `reaper` on the queue config, jobs stuck in `processing` longer than
`age_minutes` are requeued, on the assumption their worker died. A running job
heartbeats automatically (four times per `age_minutes`), so a long job that is
still alive is not reaped. There is no heartbeat code to write; set the reaper:

```yaml
queue:
  kind: Postgres
  reaper:
    age_minutes: 10
    interval_seconds: 60
```

## Execution mode

`config/<env>.yaml` decides whether jobs run in-process or through a real queue:

```yaml
workers:
  mode: BackgroundQueue     # BackgroundAsync | ForegroundBlocking
```

| Mode | Behaviour | Use for |
|---|---|---|
| `BackgroundQueue` | persisted to Redis/Postgres/SQLite, processed by a worker process | production |
| `BackgroundAsync` | in-process async, not durable | simple dev |
| `ForegroundBlocking` | runs inline, synchronously | tests — makes assertions deterministic |

Run the worker process:

```sh
cargo loco start --worker      # workers only
cargo loco start --server-and-worker
cargo loco start --all         # server + worker + the `scheduler:` jobs
```

Plain `cargo loco start` is the server only: in `BackgroundQueue` mode it
enqueues jobs and nothing runs them. If jobs enqueue but never execute, the
worker process is not running — that is the usual cause, not a code bug. Put
the right command in the README and the Procfile/Dockerfile.

A job that opens a transaction holds one pool connection until it commits,
including while it waits on HTTP calls or on a lock (`pg_advisory_xact_lock`).
Each concurrently running job can hold one, so keep `database.max_connections`
above the number of jobs that can run at once plus what the web server needs,
and do slow network I/O outside the transaction where you can. A pool that is
too small shows up as `connection pool timed out` after `connect_timeout`
(500 ms by default), on requests unrelated to the job.

## Testing

Set `ForegroundBlocking` in `config/test.yaml` so `perform_later` completes
before the assertion runs. Otherwise the test races the queue.
