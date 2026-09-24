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

## Do not hand-roll retries

A loop with `tokio::time::sleep` and an attempt counter adds no dependency and
is still wrong — it blocks a worker slot, loses state on restart, and silently
swallows the failure when the counter runs out. **Return `Err` and let the queue
retry.** Retry behaviour is queue configuration, not worker code.

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
```

If jobs enqueue but never execute in production, the worker process is not
running — that is the usual cause, not a code bug.

## Testing

Set `ForegroundBlocking` in `config/test.yaml` so `perform_later` completes
before the assertion runs. Otherwise the test races the queue.
