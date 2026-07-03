# Loco Background Processing — Verified Inventory

Area: workers/queues, scheduler, tasks. All refs verified against code on branch `release/0.17.0`.
Paths are absolute-relative to repo root `/Users/jondot/projects/loco`.

---

## 1. Background Workers (BackgroundWorker trait)

**Purpose:** Define typed background jobs enqueued via `perform_later`; executed either in-process (async/blocking) or via a persistent queue.

### Public API surface
- `trait BackgroundWorker<A>` — `src/bgworker/mod.rs:608`
  - `fn queue() -> Option<String>` (default `None`) — `mod.rs:614`
  - `fn tags() -> Vec<String>` (default empty) — `mod.rs:621`
  - `fn build(ctx: &AppContext) -> Self` — `mod.rs:625`
  - `fn class_name() -> String` (UpperCamelCase of struct name, module path stripped, via `heck`) — `mod.rs:627`
  - `async fn perform_later(ctx, args) -> Result<String>` — `mod.rs:640` (delegates to priority variant with `None`)
  - `async fn perform_later_with_priority(ctx, args, priority: Option<i32>) -> Result<String>` — `mod.rs:653`  ← **NEW, undocumented**
  - `async fn perform(&self, args: A) -> Result<()>` — `mod.rs:700`
- Return type changed: `perform_later*` returns **`Result<String>` (the job id)**, not `Result<()>`. In queue mode the id is provider-assigned; in foreground/async/no-provider a fresh `uuid::Uuid::v4` is generated — `mod.rs:661-698`.
- `Queue` enum (provider handle) — `mod.rs:68`. Key methods:
  - `enqueue(class, queue, args, tags, priority) -> Result<Option<String>>` — `mod.rs:109`
  - `register<A,W>(worker)` — `mod.rs:162`
  - `run(tags)`, `setup`, `clear`, `ping`, `shutdown`, `describe` — `mod.rs:197,274,296,321,358,341`
  - Job admin: `cancel_jobs`, `clear_jobs_older_than`, `clear_by_status`, `requeue`, `dump`, `import`, `get_jobs` (private) — `mod.rs:416,442,477,501,528,566,373`
- `enum JobStatus { Queued, Processing, Completed, Failed, Cancelled }` — `mod.rs:33` (also `#[derive(ValueEnum)]` under `cli`)
- Free fns: `converge(queue, config)` — `mod.rs:708`; `create_queue_provider(config) -> Result<Option<Arc<Queue>>>` — `mod.rs:754`

### Worker registration (Hooks)
- `Hooks::connect_workers(ctx, queue: &Queue) -> Result<()>` — `src/app.rs:422`. Users call `queue.register(MyWorker::build(ctx)).await?`.
- Worker generator injects registration into `connect_workers` (see generators).

### Worker modes (config)
- `enum WorkerMode { BackgroundQueue (#[default]), ForegroundBlocking, BackgroundAsync }` — `src/config/server.rs:72`
- `struct Workers { mode: WorkerMode }` — `src/config/server.rs:65`; field `Config.workers` — `src/config/mod.rs:73`
- Mode semantics in `perform_later_with_priority` — `mod.rs:661`:
  - `BackgroundQueue`: enqueue to configured provider (needs `queue:` config + provider feature).
  - `ForegroundBlocking`: build + `perform` inline (used for tests).
  - `BackgroundAsync`: `tokio::spawn` in same process (no external store).
- NOTE: `WorkerMode` doc-comment on `BackgroundQueue` still says "**Requires a Redis connection**" (`server.rs:74`) — STALE, Postgres/SQLite are equally valid.

### Feature flags
- `bg_redis`, `bg_pg`, `bg_sqlt` — all three in default features — `Cargo.toml:33-35`, defined `Cargo.toml:58-60`:
  - `bg_redis = ["dep:redis", "dep:ulid"]`
  - `bg_pg = ["dep:sqlx", "dep:ulid"]`
  - `bg_sqlt = ["dep:sqlx", "dep:ulid"]`
- Backend modules gated: `pg` (`mod.rs:13`), `redis` (`mod.rs:15`), `sql` shared (`bg_pg`||`bg_sqlt`, `mod.rs:17`), `sqlt` (`mod.rs:19`).

---

## 2. Shared SQL Driver trait (1.0 dedup) — `src/bgworker/sql.rs`

**Purpose:** De-duplicates the identical Postgres/SQLite queue logic. `Job` model, `JobRegistry` (worker registration + poll/run loop), `RunOpts`, and the panic-catching handler now live once in `sql.rs`; each SQL backend only supplies pool type + 3 DB ops via the `Driver` trait.

- `pub(crate) trait Driver` — `sql.rs:61`: assoc `type Pool`; `idle_count`, `dequeue(pool, tags)`, `complete_job(pool, id, interval)`, `fail_job(pool, id, error)` — `sql.rs:64-81`
- `struct Job { id, name, data(#[serde rename task_data]), status, run_at, interval, created_at, updated_at, tags, priority(#[serde default]) }` — `sql.rs:36`
- `struct RunOpts { num_workers: u32, poll_interval_sec: u32 }` — `sql.rs:53`
- `struct JobRegistry` + `register_worker` (wraps `perform` in `catch_unwind`, deserializes args) — `sql.rs:84,100`
- `JobRegistry::run<D: Driver>(pool, opts, token, tags)` — generic poll loop, spawns `num_workers` tokio tasks, `CancellationToken`-aware, `biased` select on cancel vs sleep — `sql.rs:147`
- `pg.rs`/`sqlt.rs` each `impl Driver` (`PgDriver` `pg.rs:23`, `SqliteDriver` `sqlt.rs:23`) and `Queue::run` dispatches `registry.run::<pg::PgDriver>` / `::<sqlt::SqliteDriver>` — `mod.rs:210,220`. Redis has its own independent run loop (`redis::JobRegistry::run`, `redis.rs:147`) — NOT on the `Driver` trait.

---

## 3. Priority queues (1.0 feature) — undocumented

**Purpose:** Higher `priority` (i32) dequeues first, across all three backends.

- Semantics (documented in code) — `mod.rs:104-107`: higher = more urgent; full i32 range; ties broken by earlier `run_at`, then stable job id.
- Postgres: `priority INT NOT NULL DEFAULT 0`; dequeue `ORDER BY priority DESC, run_at, id LIMIT 1 FOR UPDATE SKIP LOCKED` — `pg.rs:111,195`. **Auto-migrates** existing tables (adds `priority` column if missing) — `pg.rs:81-95`.
- SQLite: `priority INTEGER NOT NULL DEFAULT 0`; dequeue `ORDER BY priority DESC, run_at, id LIMIT 1` — `sqlt.rs:80,206`. Auto-migrates via `pragma_table_info` check — `sqlt.rs:97-110`.
- Redis: stored in a ZSET scored by `-priority` (negated so `ZRANGE` = highest first); score is priority-only to preserve full i32 range, with explicit run_at/id tie-break in `dequeue_with_conn` — `redis.rs:290-307,406`. `priority` field on `Job` — `redis.rs:49`.
- Exposed via `perform_later_with_priority`, `Queue::enqueue(...priority)`, and `import` preserves `job.priority` — `mod.rs:574,584,593`.

---

## 4. The three queue backends — user-facing differences

Config enum `QueueConfig` (serde `tag = "kind"`) — `src/config/queue.rs:7`. Selected by `queue.kind` in YAML.

### Redis — `RedisQueueConfig` (`queue.rs:16`)
- Fields: `uri`, `dangerously_flush` (default false), `queues: Option<Vec<String>>`, `num_workers` (default 2).
- **Custom/priority named queues**: `queues` list, first = most important — `queue.rs:22`. Default queues `["default", "mailer"]` — `redis.rs:928`; `get_queues` merges config queues ahead of defaults — `redis.rs:930`.
- `Worker::queue()` selects which named queue the job lands in.
- `setup()` is a no-op for Redis (no schema) — `mod.rs:277`.
- Feature: `bg_redis`.

### Postgres — `PostgresQueueConfig` (`queue.rs:31`)
- Fields: `uri`, `dangerously_flush`, `enable_logging`, `max_connections` (`db_max_conn`), `min_connections` (`db_min_conn`), `connect_timeout` (`db_connect_timeout`), `idle_timeout` (`db_idle_timeout`), `poll_interval_sec` (default 1, `pgq_poll_interval`), `num_workers` (default 2).
- Table `pg_loco_queue`; `setup()` runs `initialize_database` — `pg.rs:67`, `mod.rs:279`. Uses `FOR UPDATE SKIP LOCKED` for concurrency. NO named `queues` concept (single table).
- Feature: `bg_pg`.

### SQLite — `SqliteQueueConfig` (`queue.rs:59`)
- Same field set as Postgres (`poll_interval_sec` via `sqlt_poll_interval`, default 1) — `queue.rs:59-86`.
- Tables `sqlt_loco_queue` (+ `sqlt_loco_queue_lock`), index `idx_sqlt_queue_status_run_at` — `sqlt.rs:70-91`. `setup()` → `initialize_database` — `mod.rs:283`. No `SKIP LOCKED`; uses a lock table.
- Feature: `bg_sqlt`.

### Cross-backend admin support (all three now implemented)
`get_jobs`, `cancel_jobs_by_name`, `clear_by_status`, `clear_jobs_older_than`, `requeue`, `dump`, `import` are implemented for **all three** backends incl. Redis (`redis.rs:514,890,602,686,786` + import `mod.rs:590`).
- STALE DOCSTRINGS: `Queue::cancel_jobs`/`clear_jobs_older_than`/`clear_by_status`/`requeue`/`import` error docs still say "If the Redis provider is selected, it will return an error stating that ... is not supported" (`mod.rs:413-414,438-440,474-476,498-500,563`). Code no longer errors for Redis — comments are outdated.
- `converge()` honors `dangerously_flush` (clears queue on boot) for all three — `mod.rs:708-745`.

---

## 5. Scheduler — `src/scheduler.rs`

**Purpose:** Cron-like scheduler that runs registered tasks or shell commands; wraps `tokio_cron_scheduler`, accepts English or cron syntax.

### Public API / config types
- `struct Config { jobs: HashMap<String, Job>, output: Output }` (`#[serde(deny_unknown_fields)]`) — `scheduler.rs:56`. Exposed as `Config.scheduler: Option<scheduler::Config>` — `src/config/mod.rs:91`.
- `struct Job { run: String, shell: bool, run_on_start: bool, cron(#[serde rename "schedule"]): String, tags: Option<Vec<String>>, output: Option<Output> }` — `scheduler.rs:67`
- `enum Output { Silent, STDOUT(#[default]) }` — `scheduler.rs:137`
- `struct Scheduler { jobs, binary_path, default_output, environment }` — `scheduler.rs:121`
- `struct Spec { name: Option<String>, tag: Option<String> }` — filter — `scheduler.rs:130`
- `Scheduler::from_config::<H>(path, env)` — `scheduler.rs:219`; `::new::<H>(&Config, env)` (validates each non-shell job's task name is registered) — `scheduler.rs:240`; `by_spec(&Spec)` — `scheduler.rs:272`; `async run(self)` — `scheduler.rs:299`
- `struct JobDescription { command, output, environment }` + `run() -> io::Result<Output>` — `scheduler.rs:149,195`
- English→cron via `english_to_cron` when input doesn't start with `*`/digit — `scheduler.rs:306`; cron format is **7-field incl. seconds & year**, UTC.
- Execution: each fire spawns a **subprocess** through `/bin/sh -c` (or `cmd.exe /C` on Windows), with `LOCO_ENV` env propagated — `scheduler.rs:198-209,358`. `run_on_start: true` adds a one-shot at duration 0 — `scheduler.rs:317`.
- Shutdown: `run()` blocks on `ctrl_c` then `sched.shutdown()` — `scheduler.rs:349-353`.

### Env vars
- `SCHEDULER_CONFIG` — path to dedicated `scheduler.yaml` when using `start --all` (read at boot; see docs) — used in `src/cli.rs`/boot, documented in scheduler.md.
- `LOCO_ENV` propagated to job subprocesses.

---

## 6. Tasks — `src/task.rs`

**Purpose:** Ad-hoc CLI-invokable operations (data fixes, reports) with typed access to `AppContext`; also invokable by the scheduler.

### Public API
- `trait Task { fn task(&self) -> TaskInfo; async fn run(&self, ctx, vars) -> Result<()> }` — `task.rs:75`
- `struct TaskInfo { name, detail }` — `task.rs:68`
- `struct Vars { cli: BTreeMap<String,String> }` + `from_cli_args`, `cli_arg(key)` — `task.rs:13,35,58`
- `struct Tasks { registry: BTreeMap<String, Box<dyn Task>> }` + `list`, `names`, `run(ctx, name, vars)`, `register(task)` — `task.rs:84,91,97,110,120`. Register by same name overrides (BTreeMap insert).
- Registration hook: `Hooks::register_tasks(tasks: &mut Tasks)` — `src/app.rs:425`.

---

## 7. CLI / Generators

### `cargo loco` subcommands (`src/cli.rs`)
- `start [OPTIONS]` — `-w/--worker [tags...]`, `-s/--server-and-worker`, `--all`, `--scheduler` — `cli.rs:67-81`. Start modes incl. `WorkerAndScheduler`, `ServerAndScheduler` — `cli.rs:736-747`.
- `task <NAME> [PARAMS k:v ...]` — run a task — `cli.rs:796`; list tasks with bare `task`.
- `scheduler [--config <path>] [--name <n>] [--tag <t>] [--list]` — `cli.rs:122-130,801-808` → `run_scheduler`.
- `jobs <COMMAND>` — `JobsCommands` enum `cli.rs:600`:
  - `cancel --name`, `tidy`, `purge [--max-age 90] [--status ...] [--dump <path>]`, `dump [--status] [-f folder]`, `import -f <file>`, `requeue [--from-age 0]` — `cli.rs:600-647`, dispatched `cli.rs:1252-1291`.
  - NOTE: `requeue` exists in code but is **missing from the `jobs --help` snippet** in workers.md.
- Worker start gated on `WorkerMode::BackgroundQueue` — `src/boot.rs:121,134`.

### Generators (`loco-gen`)
- `Component::{Task{name}, Scheduler{}, Worker{name}}` — `loco-gen/src/lib.rs:284-289`, dispatched `lib.rs:353-361`. CLI mapping `src/cli.rs:467-469`.
- Templates:
  - `worker.t` — creates `src/workers/<name>.rs`, injects `pub mod` into `workers/mod.rs` AND `queue.register(...)` after `fn connect_workers` in `app.rs` — `loco-gen/src/templates/worker/worker.t`. Generated stub implements `build`, `class_name` (hardcoded), `tags`, `perform`.
  - `task.t` — creates `src/tasks/<name>.rs`, injects `pub mod` + `tasks.register(...)` before `// tasks-inject` — `templates/task/task.t`.
  - `scheduler.t` — creates `config/scheduler.yaml` (skeleton with shell + task job examples) — `templates/scheduler/scheduler.t`.
- Worker generator also emits a test template (`tests/templates/worker.rs`).

---

## 8. Doc coverage ratings ("only VERIFIED docs")

### `docs-site/content/docs/processing/workers.md` — **STALE / THIN on 1.0**
- ACCURATE: modes, three backends, tag filtering, register in `connect_workers`, generator, CLI jobs overview, testing pattern, `class_name()`.
- MISSING: **priority queues entirely** (`priority` arg, `perform_later_with_priority`) — undocumented.
- STALE: `perform_later` documented as returning `Result<()>` (`workers.md:234`) — code returns `Result<String>` (job id) — `mod.rs:640`.
- MISSING backend config knobs: `enable_logging`, `min_connections`/`max_connections`, `connect_timeout`, `idle_timeout`, `poll_interval_sec` (PG/SQLite); Redis `queues` (named/priority queues). Only `num_workers`/`dangerously_flush`/`uri` shown.
- STALE CLI: `jobs` help snippet (`workers.md:294-300`) omits `requeue`.
- MISSING: `Worker::queue()` usage (custom/named queues) — mentioned in trait list only, no example; Redis default queues `["default","mailer"]` unmentioned.
- MISSING (1.0 internal but worth a note): shared `Driver` trait / SQL dedup — not user-facing but explains PG/SQLite parity.

### `docs-site/content/docs/processing/scheduler.md` — **ACCURATE (minor)**
- Matches code: dedicated file vs env config, `SCHEDULER_CONFIG`, `output`/`silent`, `shell`, `run_on_start`, `tags`, English+cron (7-field UTC), `--list/--name/--tag/--config`, subprocess + env propagation, graceful shutdown.
- Minor: config example uses `run_task` with `shell` omitted (defaults false = task) — consistent. No significant discrepancies found.

### `docs-site/content/docs/processing/task.md` — **ACCURATE / THIN**
- Matches: generator, run with params (`k:v`), `cli_arg`, list, manual create + `register_tasks`.
- THIN: "Listing All Tasks" text says "tasks that have been executed" (`task.md:79`) — actually lists **registered** tasks (`Tasks::list`), minor wording bug. No mention that re-registering a name overrides.

---

## 9. 1.0-relevant notes
- **bgworker dedup**: new `pub(crate) trait Driver` + shared `JobRegistry`/`Job`/`RunOpts` in `sql.rs`; PG/SQLite reduced to `impl Driver`. Redis kept separate. Behavior parity for PG/SQLite now guaranteed by shared run loop.
- **Priority queues**: full-i32 priority across all 3 backends, with auto-migration of the `priority` column for existing PG/SQLite deployments.
- **API change**: `perform_later`/`perform_later_with_priority` now return `Result<String>` (job id); new `perform_later_with_priority`. Docs still show old `Result<()>`.
- **Redis admin parity**: cancel/clear/requeue/get_jobs/import now work for Redis; several `Queue` docstrings claiming "Redis not supported" are stale.
- **Default worker mode** is `BackgroundQueue` (`server.rs:75`), but its doc-comment wrongly says it requires Redis.
</content>
