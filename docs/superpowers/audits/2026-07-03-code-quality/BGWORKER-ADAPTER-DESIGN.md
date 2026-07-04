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
  Highest value, lowest risk (no SQL touched).
- **Phase 2 (assessed, then REJECTED — see Outcome):** hoist the dialect-parameterized
  CRUD bodies from pg/sqlt into `sql.rs`.

## OUTCOME (2026-07-04)

**Phase 1 — SHIPPED (commit 5cb1c720).** The `QueueProvider` trait + `Queue` newtype
facade land; 13× 4-way match dispatch gone; worker-erasure deduplicated; third-party
backends now implementable via `Queue::from_provider`. Gate green: fmt, clippy
`-D warnings` (all backend combos), 60/60 bgworker tests (pg+sqlt+redis containers),
monitoring readiness, zero snapshot drift.

**Honest LOC reality:** Phase 1 is **+189 LOC** across the module (mod.rs 967→891, but
four explicit ~14-method trait impls outweigh the terse match arms). This rewrite is an
**architecture/extensibility win, not a line-count win** — it passes Jondot's
"cleaner, precise, clearer, extensible" bar (adding an operation is now one trait method
+ impls, not edits to 13 match blocks; a new backend is one `impl`), but it does **not**
reduce lines, and it was wrong of me to imply it would.

**Phase 2 — REJECTED on a concrete spike (prove-why-not, and this time the "not" holds).**
The pg/sqlt CRUD bodies (`enqueue`, `cancel_jobs_by_name`, `clear_by_status`,
`clear_jobs_older_than`, `requeue`, `get_jobs`) differ only in dialect tokens, so a
generic-over-executor hoist *looks* like free LOC. It is net-negative:

- **The generic-executor pattern carries heavy bounds.** The existing `sql::ping`
  precedent needs a 3-line `where E: sqlx::Executor<'e>, <E::Database as Database>::
  Arguments: IntoArguments<E::Database>` clause to share a **one-line** body, plus a
  per-backend wrapper each. For `cancel_jobs_by_name`: 2×~8 lines of dead-simple code
  → ~14 shared (mostly bounds) + 2×3 wrappers = **~20 lines, and more indirection.**
- **It regresses static SQL into dynamic SQL.** Each hoist turns a `sqlx::query("…literal…")`
  into `sqlx::query(AssertSqlSafe(format!("…{table}…{now}…")))` — widening the
  string-interpolation surface for a class of query that today is a self-evidently-safe
  literal. Worse on both LOC and safety.
- Each function has a **different signature/shape**, so they can't amortize one generic;
  the date-math ones (`requeue`/`get_jobs`/`clear_jobs_older_than`) need extra per-dialect
  cutoff fragments, and `clear_by_status` (`= ANY($1)` bind vs `IN (…)` inline) /
  `complete_job`/`fail_job` (`json_patch` vs `|| ::jsonb`) are genuinely different logic.

Verdict: the CRUD duplication is real but each function is short, static-SQL, tested, and
correct at a glance. Replacing it with a dialect-fragment abstraction makes every query
**harder** to read for a worse LOC count. Leaving it per-driver is the cleaner outcome.
`dequeue`/`initialize_database`/`connect` stay per-driver forever (genuinely different).

## Gate (per phase)

`cargo fmt --check` · `cargo clippy --features testing,with-db,bg_pg,bg_sqlt,cache_inmem,bg_redis --lib --tests -- -D warnings`
· targeted `cargo test` for `bgworker::` (pg/sqlt/redis) incl. insta snapshots. Behavior
must be identical: **zero snapshot diffs**.
