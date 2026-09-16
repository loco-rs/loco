---
title: The background-processing model
description: Why perform_later works unmodified against Redis, Postgres, or SQLite, how the shared Driver trait keeps the two SQL backends in lockstep, and what priority and worker modes buy you.
sidebar:
  order: 5
---

Loco lets you write one `BackgroundWorker` implementation and one `perform_later` call site, then choose — by config, not by code change — whether jobs are durably queued in Redis, Postgres, or SQLite, or not durably queued at all. This page explains the design that makes that swap safe, not the step-by-step of adding a worker (that's [Add a background worker](/docs/how-to/add-worker)) or the exhaustive config keys (that's the [Configuration reference](/docs/reference/configuration#queue) and [Choose a queue backend](/docs/how-to/choose-queue-backend)).

## One trait, one call site, three backends

```rust
#[async_trait]
pub trait BackgroundWorker<A> {
    fn build(ctx: &AppContext) -> Self;
    async fn perform(&self, args: A) -> Result<()>;
    // + queue(), tags(), class_name(), perform_later(), perform_later_with_priority(),
    //   perform_all_later(), perform_all_later_with_priority()
}
```

You implement `perform`, register the worker in `connect_workers`, and enqueue work with `MyWorker::perform_later(&ctx, args).await?`. Nothing in that call references which backend is active — that's decided entirely by `queue.kind` in config (`Redis` | `Postgres` | `Sqlite`), read at boot by `create_queue_provider`. This is the same "config over code" bias covered in [Why batteries included](/docs/explanation/why-batteries-included), applied to durability and delivery semantics: swapping backends is an operational decision (what's already running in your infrastructure, what latency/throughput profile you need), not a rewrite.

`perform_later` (and its sibling `perform_later_with_priority`) returns `Result<String>` — the job's id — rather than `Result<()>`. That return value matters because it's what you'd hand to `cargo loco jobs cancel`/`requeue` or log for later correlation; treat any `perform_later` call site that discards its return value as intentionally choosing not to track the job, not as the only option.

The batch pair, `perform_all_later` and `perform_all_later_with_priority`, follows the same contract for many jobs at once: one `Vec` of ids back, in input order, and one round trip to the queue instead of one per job. Each built-in provider implements `QueueProvider::enqueue_batch` atomically — a single transaction on Postgres/SQLite, a `MULTI`/`EXEC` pipeline on Redis — so a batch either fully enqueues or leaves nothing behind. The trait has a default `enqueue_batch` that loops over `enqueue`, so a third-party provider keeps compiling and gets the batch semantics without the atomicity until it overrides the method. This mirrors Rails' `ActiveJob.perform_all_later`, which falls back to individual enqueues on adapters without bulk support.

## The two SQL backends share one implementation

Postgres and SQLite queueing used to be two independent, parallel implementations that had to be kept in sync by hand. As of the 1.0 line they're de-duplicated behind one internal `Driver` trait:

```rust no-syntax-check="trait sketch with `...` standing in for the real types"
pub(crate) trait Driver {
    type Pool;
    fn idle_count(&self) -> ...;
    async fn dequeue(pool: &Self::Pool, tags: &[String]) -> ...;
    async fn complete_job(pool: &Self::Pool, id: ..., interval: ...) -> ...;
    async fn fail_job(pool: &Self::Pool, id: ..., error: ...) -> ...;
}
```

The `Job` model, the polling/registration loop (`JobRegistry`), panic-catching around `perform`, and the run-loop machinery all live once, generic over `Driver`. `PgDriver` and `SqliteDriver` only need to supply the three DB operations above plus a pool type — everything else (worker registration, tag filtering, graceful cancellation) is shared code, not two copies that can drift. This is why Postgres and SQLite have identical *behavior* (same admin operations, same priority semantics, same job lifecycle) even though the underlying SQL is necessarily different — one uses `FOR UPDATE SKIP LOCKED` for concurrent dequeue, the other simulates it with a lock table since SQLite has no equivalent. Redis, being architecturally different (no SQL, no row locking), keeps its own independent run loop rather than implementing `Driver` — but is still held to the same external contract (the same `Queue` API, the same job lifecycle, the same admin operations) from the outside.

That shared contract is what lets `cargo loco jobs cancel|tidy|purge|dump|import|requeue` work identically regardless of which backend is configured — including Redis, which historically lagged the SQL backends on admin-operation support but is now at parity.

## Priority: one semantic, three storage strategies

All three backends dequeue by priority first, then by age: a higher `i32` priority value is more urgent, ties break by earlier `run_at`, then by a stable job id. How each backend *stores* that ordering differs with its storage model, which is worth understanding since it explains the backends' relative strengths:

- **Postgres / SQLite** add a `priority` column and an `ORDER BY priority DESC, run_at, id` on dequeue (existing pre-1.0 tables are auto-migrated to add the column). This is a natural fit for a row store with a query planner.
- **Redis** has no query planner to lean on, so priority is encoded structurally: jobs live in a sorted set (ZSET) scored by *negative* priority, so the highest-priority job sorts first under `ZRANGE`'s ascending order — with `run_at`/id used as an explicit tie-break in the dequeue logic, since the score alone can't carry three levels of ordering.

Redis additionally supports **named queues** (`queue.queues: [high, low, ...]`, first = most important) with two independent workers backed by the default `["default", "mailer"]` queues, and a `Worker::queue()` override to route a specific worker's jobs into one. This is a coarser-grained tool than per-job priority — named queues partition *which pool of workers* picks up a job, while `priority` decides ordering *within* that pool — and the two compose (a named queue can still be priority-ordered internally).

## Worker modes: trading durability for simplicity

`workers.mode` is a separate axis from the queue backend — it decides whether a persistent queue is even in the picture:

| Mode | Durable across restarts? | Where jobs run | Typical use |
|---|---|---|---|
| `BackgroundQueue` (default) | yes | a separate worker process/thread, dequeuing from the configured `queue:` backend | production |
| `ForegroundBlocking` | n/a — runs inline | the calling request/task, synchronously | tests, where you want deterministic execution before asserting on side effects |
| `BackgroundAsync` | no — lost on crash | `tokio::spawn` in the same process | low-stakes, best-effort work where standing up a queue backend isn't worth it |

The reason this is a mode switch rather than a code difference is the same reason the backend is a config switch: `perform_later` and `perform` don't change, so a worker written and tested under `ForegroundBlocking` behaves identically once the app is switched to `BackgroundQueue` in production — the only thing that changes is *when* and *where* `perform` actually executes, not its logic.

## Choosing a backend

There's no universally correct choice — the three backends trade off along real infrastructure axes:

- **Redis** — lowest latency, named/priority queues, no schema to manage; the right default if Redis is already part of your stack.
- **Postgres** — no new infrastructure if your app's primary database is already Postgres, and `FOR UPDATE SKIP LOCKED` gives solid concurrent-worker throughput.
- **SQLite** — zero extra infrastructure at all, good for small deployments or local development; the lock-table fallback for concurrency makes it less suited to a large number of concurrent workers than the other two.

See [Choose a queue backend](/docs/how-to/choose-queue-backend) for the concrete config for each, and the [Configuration reference](/docs/reference/configuration#queue) for every field and its default.
