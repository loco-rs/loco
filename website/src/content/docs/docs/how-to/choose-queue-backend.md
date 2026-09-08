---
title: Choose a queue backend
description: Pick Redis, Postgres, or SQLite for background jobs, configure it, and pick a worker mode.
sidebar:
  order: 21
---

Goal: decide how background jobs (see [Add a background worker](/docs/how-to/add-worker)) are enqueued, stored, and processed, and configure it.

## 1. Pick a worker mode

`workers.mode` controls whether jobs go through a persistent queue at all:

```yaml
# config/development.yaml
workers:
  mode: BackgroundQueue # default. Options: BackgroundQueue | ForegroundBlocking | BackgroundAsync
```

| Mode | Needs a `queue:` backend? | Behavior |
|---|---|---|
| `BackgroundQueue` (default) | yes | Enqueues to the configured provider; a separate worker process (or thread) dequeues and runs jobs. Survives restarts. |
| `ForegroundBlocking` | no | Runs the job inline, blocking the caller until it finishes. Used in tests. |
| `BackgroundAsync` | no | `tokio::spawn`s the job in the same process. No external store — jobs are lost on crash/restart. |

If you only need `BackgroundAsync` or `ForegroundBlocking`, you can stop here — skip the `queue:` config entirely.

The `loco new` wizard asks for this up front, offering `Async`, `Queue: Redis`, `Queue: Postgres`, `Queue: SQLite`, or `Blocking`; picking one of the three `Queue: *` options wires up the matching `workers.mode: BackgroundQueue` config, `queue.kind`, and Cargo feature (`worker` for Postgres/SQLite, `worker_redis` for Redis) for you.

## 2. Pick and configure a backend

All three backends share the same `perform_later` API and priority semantics; switching is a config change, not a code change. Set `queue.kind` in your environment YAML:

### Redis

```yaml
queue:
  kind: Redis
  uri: "<%= get_env(name='REDIS_URL', default='redis://127.0.0.1') %>"
  dangerously_flush: false # clears the queue on boot — dev/test only
  queues: [high, low] # optional: named/priority queues, first = most important
  num_workers: 2 # concurrent job handlers
```

Requires the `worker_redis` Cargo feature. Unlike Postgres/SQLite, this one is **not** in the default feature set — enable it explicitly (`worker_redis` implies `worker`, so you don't need to list both):

```toml
loco-rs = { version = "...", features = ["worker_redis"] }
```

`setup()` is a no-op for Redis — there's no schema to create.

### Postgres

```yaml
queue:
  kind: Postgres
  uri: "<%= get_env(name='PGQ_URL', default='postgres://localhost:5432/mydb') %>"
  dangerously_flush: false
  enable_logging: false
  max_connections: 20
  min_connections: 1
  connect_timeout: 500 # ms
  idle_timeout: 500 # ms
  poll_interval_sec: 1
  num_workers: 2
```

Requires the `worker` Cargo feature (on by default — no extra `features = [...]` needed for a plain `loco-rs` dependency). Jobs live in a `pg_loco_queue` table; the table (and a `priority` column, for pre-1.0 tables) is created/migrated automatically on boot.

### SQLite

```yaml
queue:
  kind: Sqlite
  uri: "<%= get_env(name='SQLTQ_URL', default='sqlite://loco_development.sqlite?mode=rwc') %>"
  dangerously_flush: false
  poll_interval_sec: 1
  num_workers: 2
  # remaining keys identical to Postgres
```

Requires the `worker` Cargo feature (on by default) — the same flag that gates the Postgres backend above; both share the `sqlx`-based provider and are picked between at runtime by `queue.kind`. Uses `sqlt_loco_queue` (+ a lock table, since SQLite has no `SELECT ... FOR UPDATE SKIP LOCKED`).

The upshot: `worker` covers Postgres and SQLite queues (already in the default feature set), while `worker_redis` adds the Redis queue on top. Which backend actually runs is a runtime choice — `queue.kind: Postgres | Sqlite | Redis` — not a per-database feature flag. For the exhaustive key list (defaults included), see [Configuration reference → queue](/docs/reference/configuration#queue). For flag names and how to trim the default feature set, see [Feature flags reference](/docs/reference/feature-flags).

## 3. Run the worker process

```sh
cargo loco start --worker           # dedicated worker process
cargo loco start --server-and-worker # server + worker, one process
```

See [Add a background worker](/docs/how-to/add-worker#5-run-the-worker-process) for tag filtering.

## Priority queues

All three backends support per-job priority: higher `priority` (a full `i32`) is dequeued first; ties break by earlier `run_at`, then by job id. Set it with `perform_later_with_priority` instead of `perform_later`:

```rust
DownloadWorker::perform_later_with_priority(&ctx, args, Some(100)).await?;
```

To enqueue many jobs in one round trip, use `perform_all_later` (all at the default priority) or `perform_all_later_with_priority` (each job paired with its own `Option<i32>`). All three backends enqueue the batch atomically — one transaction on Postgres/SQLite, one `MULTI`/`EXEC` block on Redis — so either every job is enqueued or none are. See [Add a background worker](/docs/how-to/add-worker#enqueue-many-jobs-at-once).

Redis additionally supports **named** queues via `queue.queues` — `Worker::queue()` picks which named queue a job lands in, and the config list order sets each queue's priority (first = most important). The default named queues are `["default", "mailer"]`.

## Managing jobs from the CLI

Once `worker` (Postgres/SQLite) or `worker_redis` (Redis) is enabled, `cargo loco jobs` is available for all three backends — including Redis, which now fully supports admin operations (cancel, clear, requeue, dump/import are no longer Postgres/SQLite-only):

```sh
cargo loco jobs cancel --name <NAME>
cargo loco jobs tidy                 # delete completed/cancelled jobs
cargo loco jobs purge --max-age 90    # delete old failed/cancelled jobs
cargo loco jobs dump -f <folder>
cargo loco jobs import -f <file>
cargo loco jobs requeue --from-age 0  # move stuck "processing" jobs back to "queued"
```

See the full flag list in the [CLI reference](/docs/reference/cli#2-3-jobs-subcommands).

### Automatic requeue (reaper)

Running `cargo loco jobs requeue` by hand recovers jobs stranded in `processing` after a worker crash, but nothing does this automatically by default. To have the running worker process do it periodically, opt in with a `reaper` block under `queue:` (all three backends support it):

```yaml
queue:
  kind: Postgres
  uri: "<%= get_env(name='PGQ_URL', default='postgres://localhost:5432/mydb') %>"
  # ...
  reaper:
    age_minutes: 10 # requeue jobs stuck in "processing" for longer than this
    interval_seconds: 60 # optional, default 60 — how often to sweep
```

Leaving `reaper` unset (the default) keeps prior behavior unchanged — no background sweep runs, and stranded jobs stay in `processing` until you run `cargo loco jobs requeue` yourself.

## Choosing between the three

- **Redis** — lowest latency, named/priority queues, no extra schema. Good default if you already run Redis.
- **Postgres** — no extra moving part if your app's database is already Postgres; `FOR UPDATE SKIP LOCKED` gives solid concurrency.
- **SQLite** — zero extra infrastructure for small deployments or local dev; uses a lock table instead of `SKIP LOCKED`, so it's less suited to high worker concurrency.
