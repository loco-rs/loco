# bgworker Adapter Interface — design of record (2026-07-04)

**Origin:** Jondot pushed back on the earlier "bgworker full unification is unsafe"
rejection. That rejection conflated two different things and condemned the whole
module for a property of *one method*:

- **Unifying `dequeue`'s SQL body** across Postgres (`FOR UPDATE SKIP LOCKED`) and
  SQLite (advisory `sqlt_loco_queue_lock`) — genuinely unsafe (double-processing).
- **Re-evaluating the abstract interface**, ActiveJob-style — the real rewrite,
  which keeps each backend's `dequeue` body separate *behind the trait*, exactly as
  ActiveJob keeps Sidekiq's and Solid Queue's internals separate behind its adapter API.

The second is what was asked for and what this delivers. The `dequeue` concurrency
finding is preserved and simply irrelevant to the interface.

## What's wrong today

- `mod.rs` (~967 lines): the `Queue` **enum** forwards **every** operation through a
  hand-written `match self { Redis→… Postgres→… Sqlite→… None→… }`. That block is
  repeated **13×** (`enqueue`, `register`, `run`, `setup`, `clear`, `ping`, `get_jobs`,
  `cancel_jobs`, `clear_by_status`, `clear_jobs_older_than`, `requeue`, `import`,
  `describe`, `shutdown`).
- A partial `Driver` trait already abstracts the 4 hot-loop ops
  (`dequeue`/`complete_job`/`fail_job`/`idle_count`) — proof the seam belongs here; it
  was just never extended to the other ~10 ops.
- `redis::JobRegistry::register_worker` + its `JobHandler` typedef are **byte-identical**
  to `sql.rs`'s — duplicated worker-erasure logic.

## Design: `QueueProvider` trait + `Queue` newtype facade

`Queue` becomes `pub struct Queue(Arc<dyn QueueProvider>)`. Because the public type
*name* is preserved, every external caller compiles unchanged:
`AppContext.queue_provider: Option<Arc<bgworker::Queue>>`, `queue.run/register/ping/
cancel_jobs/clear_by_status/requeue`, and `create_provider(...) -> Queue`. **The only
breaking edit is `Queue::None` → `Queue::empty()`** (sole external site:
`monitoring.rs` test). Undocumented variant-destructuring (`Queue::Postgres(pool,..)`)
breaks — effectively nobody.

Object-safe trait (via `async_trait`, already a dep), takes `serde_json::Value` at the
boundary — the enum already did `to_value(args)?` before dispatch, so the generic
`A: Serialize` simply stays up on the newtype:

```rust
#[async_trait]
pub trait QueueProvider: Send + Sync {
    async fn enqueue(&self, class: String, queue: Option<String>, args: JsonValue,
                     tags: Option<Vec<String>>, priority: Option<i32>) -> Result<Option<String>>;
    async fn register_handler(&self, name: String, handler: JobHandler) -> Result<()>;
    async fn run(&self, tags: Vec<String>) -> Result<()>;
    async fn setup(&self) -> Result<()>;
    async fn clear(&self) -> Result<()>;
    async fn ping(&self) -> Result<()>;
    async fn get_jobs(&self, status: Option<&Vec<JobStatus>>, age_days: Option<i64>) -> Result<Vec<Job>>;
    async fn cancel_jobs_by_name(&self, name: &str) -> Result<()>;
    async fn clear_by_status(&self, status: Vec<JobStatus>) -> Result<()>;
    async fn clear_jobs_older_than(&self, age_days: i64, status: Option<&Vec<JobStatus>>) -> Result<()>;
    async fn requeue(&self, age_minutes: &i64) -> Result<()>;
    fn describe(&self) -> String;
    fn shutdown(&self) -> Result<()>;
}
```

- **Worker erasure hoisted once** to `mod.rs::erase_worker<A,W>() -> JobHandler`
  (the closure that was duplicated in sql + redis). Registries drop their generic
  `register_worker`; gain a non-generic `insert_handler`. `Queue::register<A,W>` stays
  a generic inherent method: `erase_worker(worker)` → `provider.register_handler(...)`.
- **Backends become structs** holding what the enum tuple held:
  `PgQueue { pool, registry, run_opts, token }`, `SqliteQueue { … }`,
  `RedisQueue { … }` (its own `redis::RunOpts`/registry), `NoopQueue`.
- **`NoopQueue`** reproduces the current `None`/`_` arm behavior *exactly*, per method:
  `enqueue→Ok(None)`, `register/run/setup/clear/ping/shutdown→Ok(())`,
  `describe→"no queue"`, and `get_jobs/cancel/clear_by_status/clear_jobs_older_than/
  requeue/import→Err("provider not configured")`.
- Backend `impl`s **call the existing free functions** (`pg::enqueue`, …). **No SQL
  changes in this phase** — pure dispatch restructure. Behavior-identical; validated
  against the existing 60+ backend tests + snapshots.

## Scope

- **Phase 1 (this change):** the trait + newtype + dispatch collapse + erasure hoist.
  Highest value, lowest risk (no SQL touched). Expected mod.rs ~967 → ~350–450 LOC.
- **Phase 2 (assessed separately, only if Phase 1 lands clean):** hoist the
  dialect-parameterized CRUD bodies (`enqueue`, `cancel_jobs_by_name`, `clear_by_status`,
  `clear_jobs_older_than`, `requeue`, `get_jobs`) from pg/sqlt into `sql.rs`. They differ
  only in dialect tokens (`NOW()`/`CURRENT_TIMESTAMP`, `$n`/`?`, `= ANY`/`IN`, date math,
  JSON-merge ops). Real LOC, but genuine SQL-injection/correctness risk per fragment —
  each must be verified against snapshots. `dequeue`/`initialize_database`/`connect` stay
  per-driver forever (genuinely different).

## Gate (per phase)

`cargo fmt --check` · `cargo clippy --features testing,with-db,bg_pg,bg_sqlt,cache_inmem,bg_redis --lib --tests -- -D warnings`
· targeted `cargo test` for `bgworker::` (pg/sqlt/redis) incl. insta snapshots. Behavior
must be identical: **zero snapshot diffs**.
