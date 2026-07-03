# Spike S2 — H3: Could `apalis` replace Loco's hand-rolled multi-backend queue?

**Hypothesis under test** (from `iter1/A5-bgworker.md`, "Library hypotheses"):
> `apalis` (multi-backend async job-processing framework with `apalis-sql`
> (Postgres/SQLite/MySQL) and `apalis-redis` backends, built-in retry/backoff,
> cron) could replace the ~5,646 LOC hand-rolled poll/dequeue/ack/retry loop
> across `src/bgworker/{mod,sql,pg,sqlt,redis}.rs`.

**Verdict: DOESN'T-FIT** (see line at bottom). Evidence below.

---

## 1. Incumbent surface (read in full: `mod.rs`, `sql.rs`; `pg.rs`/`sqlt.rs`/`redis.rs`
already read in full for A5)

Loco's `Queue` enum (`src/bgworker/mod.rs:68-91`) is **one runtime value** that
hosts an arbitrary, dynamically-growing set of job "classes" registered by
*string name*:

- `Queue::register::<Args, W>(worker)` (`mod.rs:161-189`) calls
  `JobRegistry::register_worker(W::class_name(), worker)` (`sql.rs:100-137`,
  duplicated for Redis at `redis.rs:99-134`), inserting into a
  `HashMap<String, JobHandler>` (`sql.rs:84-96`).
- `Queue::enqueue(class: String, queue: Option<String>, args: A, tags:
  Option<Vec<String>>, priority: Option<i32>)` (`mod.rs:109-154`) takes the
  worker name as a **runtime `String`**, not a Rust type parameter — any
  caller anywhere in the app can enqueue any registered class by name.
- A **single shared poll loop** (`sql.rs:147-238`) looks up the handler by
  job-row `name` at dequeue time and dispatches dynamically
  (`handlers.get(&job.name)`, `sql.rs:185`).
- Priority: explicit full-`i32` semantics, tie-broken by `run_at` then
  **stable job id** (`mod.rs:104-107`).
- Tags: `Vec<String>` per job, worker declares `tags()` and only dequeues
  jobs matching (`BackgroundWorker::tags()`, `mod.rs:618-623`; tag filtering
  implemented per-backend, e.g. `pg.rs:184-192`, `sqlt.rs:193-204`,
  `redis.rs` ZSET scan).
- Admin surface on the *same* `Queue` value: `dump`/`import` to/from YAML
  (`mod.rs:519-605`), `clear_jobs_older_than`, `clear_by_status`,
  `cancel_jobs_by_name`, `requeue` (`mod.rs:409-517`) — all keyed by Loco's
  own `JobStatus` enum (`Queued/Processing/Completed/Failed/Cancelled`,
  `mod.rs:33-44`).

This is the exact shape a "drop-in" would need to match: **one queue value,
N job classes registered/enqueued by string name, shared admin ops, native
priority + tags.**

## 2. apalis — real version, real API (verified, not assumed)

- **crates.io**: `apalis` / `apalis-sql` / `apalis-redis` / `apalis-core` all
  report `max_stable_version: 0.7.4`. `newest_version` for all four is a
  **1.0.0-rc.9 / rc.8 pre-release** — apalis is mid-rewrite of its public API
  (new `Backend`/`TaskSink`/`WorkerBuilder` architecture visible in the
  `main`-branch docs served via context7, e.g. `apalis_core::worker::builder::WorkerBuilder`,
  which does **not** match the 0.7.4 API used below). This spike targets
  **0.7.4**, the only stable, crates.io-published, non-prerelease line —
  using the rc would be testing a moving target.
- Fetched real source at git tag `v0.7.4` (github.com/apalis-dev/apalis):
  `packages/apalis-sql/src/postgres.rs`, `packages/apalis-sql/src/sqlite.rs`,
  `packages/apalis-redis/src/storage.rs`, and the SQL migrations under
  `packages/apalis-sql/migrations/{postgres,sqlite}/`.
- **Postgres/SQLite (`apalis-sql` 0.7.4)**: physical schema is one shared
  table (`apalis.jobs` / `Jobs`) with a `job_type TEXT` column
  (`packages/apalis-sql/migrations/postgres/20220530084123_jobs_workers.sql`).
  A **native `priority INTEGER DEFAULT 0` column** was added in
  `20250307001101_add_job_priority.sql`, and `apalis.get_jobs()` orders
  `ORDER BY priority DESC, run_at ASC ... FOR UPDATE SKIP LOCKED` — priority
  is genuinely native and DB-side, close to Loco's semantics (missing only
  Loco's third tie-break, stable job id).
  BUT: `PostgresStorage<T>`/`SqliteStorage<T>` is **generic over one Rust
  type `T`**; `Config::namespace` (= the `job_type` string) defaults to
  `type_name::<T>()` and is fixed at construction
  (`packages/apalis-redis/src/storage.rs:365-368` shows the same pattern for
  Redis; SQL side confirmed via `postgres.rs:150-238` where every query binds
  `self.config.namespace`/`job_type` as a fixed, single value). Each fetch
  (`apalis.get_jobs(worker_id, v_job_type, n)`) reads only **one** job_type.
  There is **no `tags` column anywhere** in the schema.
- **Redis (`apalis-redis` 0.7.4)**: grepped `storage.rs` (1,238 lines) —
  **zero occurrences of "priority"** anywhere in the file. No priority
  queue, no tags, `Config::namespace` again defaults to `type_name::<T>()`
  and keys (`ACTIVE_JOBS_LIST`, `SCHEDULED_JOBS_SET`, etc.) are all
  namespaced per-type. This is a materially *smaller* feature set than
  Loco's `redis.rs` (ZSET priority queue + Lua atomic claim + tag scan,
  `redis.rs:299-433`).
- **Admin ops**: `PostgresStorage`/`SqliteStorage` expose `len()`,
  `fetch_by_id()`, `stats()` (aggregate counts only, `postgres.rs:737-762`),
  `vacuum()` (unconditionally deletes **all** `Done` rows — no age/status
  filter, `postgres.rs:643-647`), `kill()`/`retry()` (single job by
  `worker_id`+`task_id`, not batch-by-name, `postgres.rs:675-707`), and
  `reenqueue_orphaned()` (scoped to one `job_type`, by dead-since+count,
  `postgres.rs:710-734`). **No** equivalent of Loco's `dump`/`import`
  (YAML), `clear_jobs_older_than`, `clear_by_status`, or
  `cancel_jobs_by_name` (batch, by name, any status) exists.
- **Status taxonomy differs**: apalis uses
  `Pending/Running/Done/Retry/Failed/Killed` vs. Loco's
  `Queued/Processing/Completed/Failed/Cancelled` — any admin-op reuse needs
  a status-mapping layer, not just a query retarget.

## 3. Compiled spike (real, ran successfully)

Crate: `/private/tmp/claude-501/.../scratchpad/spikes/apalis-bgworker/`
(scratch dir, not touching the Loco repo/workspace).

`Cargo.toml` (pinned to the exact tested versions):
```toml
[dependencies]
apalis = { version = "=0.7.4", features = ["tracing"] }
apalis-sql = { version = "=0.7.4", features = ["sqlite", "tokio-comp"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
futures = "0.3"
anyhow = "1"
```

`src/main.rs` follows apalis's own documented multi-type pattern (mirrors
`examples/sqlite/src/main.rs` at tag `v0.7.4` in the apalis repo): two
distinct job structs (`EmailJob`, `SmsJob`), each wrapped in its own
`SqliteStorage<T>` sharing one `SqlitePool`, each run via its own
`WorkerBuilder` under one `Monitor`.

**Build**:
```
$ cargo build
   Compiling apalis-sql v0.7.4
   Compiling apalis-core v0.7.4
   Compiling apalis v0.7.4
   Compiling apalis_spike v0.1.0 (.../spikes/apalis-bgworker)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.96s
```
(Cargo.lock confirms `apalis 0.7.4`, `apalis-sql 0.7.4`, `apalis-core 0.7.4`.)

**Run** (`cargo run`, real output):
```
[debug] rows in Jobs table (SQLite): [("01KWM2RP48HS5N0V0KSJD77T47", "apalis_spike::EmailJob", "Pending"), ("01KWM2RP481G9THSMQB46FG1CC", "apalis_spike::SmsJob", "Pending")]
[sms worker] sending to someone@example.com (priority=0)
[email worker] sending to urgent@example.com (priority=0)
```

This **proves, empirically**: both job types land in the same physical
`Jobs` table (`job_type` column distinguishes them, exactly as the migration
predicts), and both get processed — but each via its **own** statically
typed `Storage<T>`/`WorkerBuilder`, not one dynamically-dispatched-by-string
registry. Trying to write `queue.enqueue("EmailJob".to_string(), ..., args)`
against a single polymorphic `queue` value (Loco's actual API,
`mod.rs:109-116`) has **no equivalent call** in apalis 0.7.4 — the Rust type
parameter *is* the routing key, decided at compile time, not a runtime
string looked up in a map.

## 4. Requirement-by-requirement mapping

| Loco requirement | apalis 0.7.4 capability | Verdict | Cite |
|---|---|---|---|
| Multiple interchangeable backends behind one API | Yes — `apalis-sql` (pg/sqlite/mysql) + `apalis-redis`, both implement `Storage`/`Backend` | **native** | `postgres.rs:471`, `storage.rs` (redis) |
| Per-job priority | Native `i32` column + `ORDER BY priority DESC` — **but only for SQL backends** | **native (SQL) / missing (Redis)** | `20250307001101_add_job_priority.sql`; zero "priority" hits in `apalis-redis/src/storage.rs` |
| Tags / worker-side filtering | No `tags` column/concept in schema or Redis keys at all | **missing** | schema migrations above; `storage.rs` key list (`storage.rs:229-306`) |
| Enqueue-by-name dynamic dispatch (one queue, N registered classes by runtime string) | `Storage<T>`/`push()` is generic per Rust type `T`; `namespace`/`job_type` fixed at construction, not a runtime dispatch key | **missing** (would need a hand-rolled `HashMap<String,Handler>` shim on top, i.e. re-implement `sql::JobRegistry`) | `postgres.rs:313-341` (`new`), `storage.rs:365-368` (redis, same pattern) |
| Job dump/restore (YAML) | Not present; only `fetch_by_id`, `len`, `stats` (counts) | **missing** | `postgres.rs:557-585, 737-762` |
| Admin ops: `clear_jobs_older_than`, `clear_by_status`, `cancel_jobs_by_name` (batch) | `vacuum()` deletes *all* Done rows (no filter); `kill()`/`retry()` are single-job, id-scoped | **missing / partial** | `postgres.rs:643-647, 673-707` |
| Automatic crash recovery (visibility timeout) | `reenqueue_orphaned()` exists, scoped to one `job_type` + `dead_since` + `count` | **adaptable** (closer to native than Loco, which has zero automatic reaper per A5 Evidence #7) | `postgres.rs:710-734` |

## 5. Net LOC / dependency assessment

**What apalis would remove**: the raw polling-loop + row-locking mechanics
that are already well-factored in Loco's `Driver` trait for Postgres/SQLite
(`sql.rs:61-82,147-238`, ~90 shared LOC) plus each backend's
`dequeue`/`FOR UPDATE SKIP LOCKED` / lock-table code (`pg.rs:163-225`,
`sqlt.rs:157-260`) — roughly 300-400 LOC of genuinely delicate concurrency
code, for the **SQL backends only**.

**What apalis would NOT remove, and what it would ADD**:
- Redis: apalis-redis has **no priority, no tags** — Loco's actual Redis
  requirements (ZSET priority queue + Lua atomic claim + tag scan,
  `redis.rs:299-433`, ~150 LOC) are not covered at all; you'd either drop
  those features or keep hand-rolling them, i.e. **zero LOC saved on
  Redis**, only a wrapper-shim ADDED.
- A `HashMap<String, Handler>`-style dynamic-dispatch shim to replicate
  `Queue::register`/`enqueue`-by-name on top of apalis's per-type
  `Storage<T>` — effectively re-implementing `sql::JobRegistry`
  (`sql.rs:84-137`, ~150 LOC) *on top of* apalis instead of being replaced
  by it.
- A tags shim: none is possible without forking apalis-sql's own migrations
  (schema has no tags column) — a real, uncrossable gap without vendoring.
- Re-implementing all admin ops (`dump`/`import`/`clear_jobs_older_than`/
  `clear_by_status`/`cancel_jobs_by_name`, currently ~400+ LOC combined
  across `mod.rs:409-605` + backend-specific SQL in `pg.rs`/`sqlt.rs`/
  `redis.rs`) as bespoke queries against apalis's *own* schema/status
  taxonomy (`Pending/Running/Done/Retry/Failed/Killed` vs Loco's
  `Queued/Processing/Completed/Failed/Cancelled`) — no savings, plus a
  status-mapping translation layer.
- Three new dependencies (`apalis`, `apalis-sql`, `apalis-redis`, transitively
  `apalis-core`) whose **stable line (0.7.4) is being superseded by an
  in-flight 1.0.0-rc.9 rewrite** with a different core API
  (`WorkerBuilder`/`Backend`/`TaskSink`) — adopting now means either pinning
  to a line the upstream project is already moving away from, or absorbing a
  breaking rewrite later.

**Net**: removes ~300-400 LOC of SQL-backend locking mechanics, but adds
back a comparable-or-larger amount in a dynamic-dispatch shim + re-built
admin ops + Redis feature gap, while **losing** tags entirely and weakening
the "one `Queue` value, register anything by name" architecture Loco's
public API commits to. Roughly **LOC-neutral to negative**, not a net win.

## 6. Verdict

`DOESN'T-FIT — apalis@0.7.4 — native per-type Storage<T> (compile-time job
routing) cannot express Loco's runtime string-keyed enqueue/register API;
apalis-redis has zero priority/tags support; no dump/import/clear_by_status/
clear_jobs_older_than/cancel_jobs_by_name equivalents exist — incumbent
@src/bgworker/{mod.rs:68-605,sql.rs:1-246}, net LOC ~0 to +100 (removes
~300-400 LOC of SQL locking mechanics, adds back a JobRegistry-equivalent
dispatch shim + re-implemented admin ops + a Redis feature gap with no
adaptation path)`
