# Area: A1 · Boot & Lifecycle

## Scope (files reviewed, with LOC)

All files read in full:

| File | LOC | Role |
|---|---|---|
| `src/app.rs` | 712 | `SharedStore`, `AppContext`, `Hooks` trait, `Initializer` trait, 6 unit tests |
| `src/boot.rs` | 611 | `StartMode`, `BootResult`, `create_context`, `create_app`, `run_app`, `setup_routes`, `start`, `run_db`, `run_scheduler`, `shutdown_signal` |
| `src/environment.rs` | 138 | `Environment` enum, env-var resolution, config loading entry point, 3 unit tests |
| `src/initializers/mod.rs` | 5 | feature-gated module declarations |
| `src/initializers/extra_db.rs` | 33 | `ExtraDbInitializer` |
| `src/initializers/multi_db.rs` | 32 | `MultiDbInitializer` |
| `src/banner.rs` | 104 | startup banner printing |
| `src/lib.rs` | 52 | crate root, module wiring |
| `src/prelude.rs` | 57 | re-export surface for user apps |

Total 1,744 LOC (matches AREAS.md's estimates per-file).

## Scores

| KPI | Score | One-line justification w/ primary cite |
|---|---|---|
| 1. Holistic vision | 7 | Boot phases (context → converge → routes → workers → serve) read as one coherent pipeline (`boot.rs:409-514`), but `create_app`'s with-db/without-db split (`boot.rs:403-437`) and `run_app`'s 6-arm `StartMode` match (`boot.rs:456-513`) show incremental, copy-and-tweak growth rather than a single unified shape. |
| 2. Economy of concepts | 7 | One `Hooks` trait, one `Initializer` trait, one `AppContext`, one `Environment` enum — lean for the domain. Marred by `extra_db.rs`/`multi_db.rs` being near-identical initializers that could share one generic helper (Evidence #4), and by two unrelated types both named `SharedStore` (Evidence #5). |
| 3. Low LOC | 6 | `run_app`'s 6 match arms (`boot.rs:456-513`, ~58 lines) reduce to 3 orthogonal booleans; `create_app`'s two `#[cfg]` variants (`boot.rs:403-437`) are ~90% identical; `extra_db.rs`/`multi_db.rs` (65 combined lines) are ~80% identical. |
| 4. Non-brittle | 4 | `boot.rs:146` silently no-ops on an unexpected `(router, worker)` combination; `Hooks::on_shutdown` is skipped entirely in the `WorkerOnly`/pure-worker path (Evidence #1, the headline finding); `ServeParams::port: i32` (`boot.rs:77`) admits negative/out-of-range values a `u16` would reject at the type level; `environment.rs:104-123` mutates process-global env vars unsafely inside a test with no serialization guard. |
| 5. Maintainable (DDD/OOP) | 7 | `AppContext` is a clean aggregate root (`app.rs:253-273`), `Hooks` cleanly separates required vs. defaulted lifecycle points (`app.rs:281-443`). Two same-named-but-unrelated `SharedStore` types (Evidence #5) hurt the model's clarity. |
| 6. Correctness | 4 | Zero `#[test]`/`#[tokio::test]` in `boot.rs` or `src/initializers/*.rs` (verified by grep — only `app.rs` and `environment.rs` carry `mod tests`); the only integration coverage of `boot::start` (`tests/infra_cfg/server.rs:26-45`) exercises solely the `(Some(router), None)` branch, so the `on_shutdown`-skipping bug (Evidence #1) is provably untested and unnoticed. |
| 7. No reinvented wheels | 7 | `SharedStore`'s `DashMap<TypeId, Box<dyn Any+Send+Sync>>` (`app.rs:34-224`) is hand-rolled but serves a genuine need (concurrent runtime-mutable typed storage) that `http::Extensions` doesn't cover as simply — see hypothesis, low confidence. |
| **Overall** | **6** | Solid phase-pipeline architecture and a well-designed `Hooks` contract, undermined by a real lifecycle-hook contract bug with zero test coverage to catch it, avoidable duplication in `create_app`/`run_app`/initializers, and a confusing type-name collision. |

## Evidence log

1. **FACT**: In `boot::start` (`boot.rs:91-149`), the match on `(router, worker)` has three live arms. The `(Some(router), None)` and `(Some(router), Some(tags))` arms both call `H::serve(router, &app_context, &server_config)` (`boot.rs:118`, `boot.rs:127`), and `Hooks::serve`'s default implementation (`app.rs:331-351`) is the *only* place `Self::on_shutdown(&cloned_ctx)` is invoked (`app.rs:346`). The third arm, `(None, Some(tags))` — i.e. `StartMode::WorkerOnly`/`WorkerAndScheduler` (`boot.rs:495-512`), which never produces a router — instead calls `shutdown_signal().await` directly (`boot.rs:140`) and then `shutdown_and_await_queue_worker` (`boot.rs:142-144`, itself defined at `boot.rs:167-181`), neither of which calls `H::on_shutdown`. → **Judgment**: the `on_shutdown` hook, documented as "Called when the application is shutting down... perform any necessary cleanup or final actions before the application stops completely" (`app.rs:439-441`), silently never fires for pure worker processes (no HTTP server). Any app relying on `on_shutdown` for cleanup (closing external connections, flushing metrics, etc.) loses that cleanup specifically in worker-only deployments — the exact deployment mode background-job workers run in production. → **KPI 4, 6** → **Severity: High**.

2. **FACT**: `boot.rs:146` is a bare `_ => {}` catch-all on the `(router, worker)` match. Tracing `run_app` (`boot.rs:443-514`), every `StartMode` arm sets router/worker such that `(None, None)` cannot currently occur from Loco's own code — but nothing enforces that invariant at the type level (`BootResult.router`/`.worker` are both plain `Option`s, `boot.rs:64-67`), and a custom `Hooks::boot` override (explicitly supported — `app.rs:298-324` shows both a with-DB and without-DB override example) could construct a `BootResult` with both `None`. → **Judgment**: this path fails silently (no log, no error) instead of surfacing a misconfiguration. → **KPI 4** → **Severity: Medium**.

3. **FACT**: `create_app` is defined twice — once under `#[cfg(feature = "with-db")]` (`boot.rs:403-422`) and once under `#[cfg(not(feature = "with-db"))]` (`boot.rs:424-437`). The two bodies are identical except the with-db version inserts one extra line, `db::converge::<H, M>(&app_context, &app_context.config.database).await?;` (`boot.rs:415`), and takes an extra generic parameter `M: MigratorTrait`. → **Judgment**: classic `#[cfg]`-driven fork of a whole function to add one call; could be a single generic function with the migrator step feature-gated internally, removing ~15 duplicated lines and one place where the two copies can drift out of sync during future edits. → **KPI 1, 3** → **Severity: Low**.

4. **FACT**: `src/initializers/extra_db.rs:12-33` and `src/initializers/multi_db.rs:12-33` are structurally identical `Initializer` impls: both read `ctx.config.initializers` (erroring identically if absent — `extra_db.rs:19-23` vs `multi_db.rs:19-23`), both `.get()` a named key and error if missing, both `serde_json::from_value` the result and connect a DB layer, both `.layer(Extension(..))` the router. The only difference is the config key name (`"extra_db"` vs `"multi_db"`) and the connect call (`db::connect` vs `db::MultiDb::new`). Neither file has a `#[test]`. → **Judgment**: this ~25-line pattern (fetch initializer sub-config by key, error-wrap, deserialize, connect, layer) is duplicated wholesale rather than factored into one generic helper (e.g. `fn get_initializer_config<T: DeserializeOwned>(ctx, key) -> Result<T>`). → **KPI 2, 3, 6** → **Severity: Low**.

5. **FACT**: `app.rs:255` defines `pub struct AppContext { ..., shared_store: Arc<SharedStore>, ... }` where `SharedStore` (`app.rs:35-38`) is a `DashMap<TypeId, Box<dyn Any+Send+Sync>>` container. Separately, `src/controller/extractor/shared_store.rs:6` defines `pub struct SharedStore<T>(pub T);`, an axum `FromRequestParts` extractor with a completely different shape (a generic newtype, not a map). `prelude.rs:25-28` re-exports the *extractor* `SharedStore` (`shared_store::SharedStore`) but not the *container* `app::SharedStore` — a user who does `use loco_rs::prelude::*;` gets one `SharedStore` in scope and must reach into `loco_rs::app::SharedStore` (as `src/tests_cfg/app.rs:2` does) to name the other. → **Judgment**: two unrelated types sharing an identical name across two modules that most users interact with together (store a service in `after_context`, then extract it) is a real naming collision that costs a reader/writer a double-take; it is a direct symptom of the DI mechanism (container + extractor) being designed as two separate additions rather than one cohesive feature. → **KPI 1, 5** → **Severity: Medium**.

6. **FACT**: `boot.rs:303-304` carries `#[allow(clippy::cognitive_complexity)]` directly on `run_db`, an 8-arm match (`boot.rs:308-355`) dispatching `RunDbCommand` variants, each just delegating to a `db::*` function. → **Judgment**: a complexity-lint suppression on a dispatcher function is a sign it grew arm-by-arm (one per new DB subcommand) past clippy's default threshold rather than being restructured (e.g., each arm's logic already lives in `db::*`, so this could be a small dispatch table or the suppression could be scoped tighter); low functional risk but a textbook "patch-on-patch" marker per the rubric's checklist. → **KPI 1** → **Severity: Low**.

7. **FACT**: `boot.rs:310,314,318,322,326,331,340` log routine, expected DB-command entry points (`migrate:`, `down:`, `reset:`, `status:`, `entities:`, `truncate:`, `seed:`) via `tracing::warn!`, not `info!`/`debug!`. → **Judgment**: using `warn` level for expected, user-invoked CLI operations misuses log-level semantics (a `warn` should mean "something's off"); if a deployment filters logs at `warn` and above expecting only anomalies, routine migration output floods that channel. → **KPI 4** (fragile assumption about log semantics) → **Severity: Low**.

8. **FACT**: `environment.rs:104-123` (`test_resolve_env`) calls `unsafe { env::remove_var(...) }` / `unsafe { env::set_var(...) }` on `LOCO_ENV`/`RAILS_ENV`/`NODE_ENV` — process-global mutable state — with no `#[serial]`/mutex guard, while Rust's default test harness runs tests within a crate on multiple threads concurrently. Today no other test in `src/` reads these three specific vars concurrently (verified: `scheduler.rs:203` only sets `LOCO_ENV` on a spawned *child process*'s environment, not the test process's own), so this is currently latent, not actively flaky. → **Judgment**: still a fragile pattern — any future test added to this crate that reads `LOCO_ENV`/`RAILS_ENV`/`NODE_ENV` will race with this one intermittently. → **KPI 4** → **Severity: Low**.

## Patch-on-patch smells

- `#[allow(clippy::cognitive_complexity)]` on `run_db` (`boot.rs:303`) — Evidence #6.
- Two near-identical `create_app` functions split only by a `#[cfg(feature = "with-db")]` line (`boot.rs:403-437`) — Evidence #3.
- Two near-identical `Initializer` impls (`extra_db.rs`, `multi_db.rs`) that were clearly copy-pasted from one another (identical error-message shape, identical control flow) — Evidence #4.
- Silent catch-all `_ => {}` in `boot::start`'s core dispatch (`boot.rs:146`) — Evidence #2.
- Inconsistent log-level semantics accreted command-by-command in `run_db` (`boot.rs:310-353`) — Evidence #7.

## Library hypotheses

1. **Hand-rolled**: `SharedStore` (`app.rs:34-224`) — a `DashMap<TypeId, Box<dyn Any + Send + Sync>>` typed heterogeneous store with insert/remove/get/get_ref/contains.
   **Candidate crate**: none identified that is simpler *and* fits — `http::Extensions` (already transitively present via `axum`) is the obvious structural cousin but is not internally synchronized for concurrent mutation (it's a plain `HashMap`, designed for build-once/read-many request-scoped use), so it would need an external `RwLock`/`Mutex` wrapper to match `SharedStore`'s current concurrent-insert/remove semantics — likely *more* code, not less, once that wrapping is added. A crate like `anymap`/`type-map` has the same limitation.
   **Why it might be simpler**: fewer hand-rolled lines (`app.rs:62-224`), reuse of a dependency already in the tree.
   **Risk / why it might not fit**: loses the lock-free concurrent read/write DashMap gives "for free"; the current implementation is small (~50 non-doc/test LOC) and is the best-tested code in the whole area (6 dedicated unit tests, `app.rs:496-711`). **NEEDS SPIKE** — low confidence this is worth doing; flagging only because the rubric asks the question, not because it looks like a win.

## What is genuinely excellent

- `app.rs:281-443` — the `Hooks` trait models the boot lifecycle as a clean, minimal set of named phases (`load_config` → `before_routes`/`routes`/`after_routes` → `initializers` → `connect_workers`/`register_tasks` → `before_run` → `serve` → `on_shutdown`), each with a sensible no-op default except the handful that must genuinely be app-specific (`app_name`, `boot`, `routes`, `connect_workers`, `register_tasks`). This is a textbook "one integration trait, many optional extension points" design.
- `app.rs:34-224` + `app.rs:480-711` — `SharedStore` is small, well-documented (every public method has a runnable doctest), and backed by 6 focused unit tests covering insert/remove/get/get_ref/contains/clone semantics — the strongest test coverage in the whole area.
- `boot.rs:364-401` (`create_context`) is a single, clear place where all of `AppContext`'s fields get their real values (db, mailer, queue, cache, storage, shared_store), with `H::after_context` as the one documented escape hatch (`boot.rs:400`) — a clean single-source-of-truth for context assembly.
- `boot.rs:368-377` — the `RUST_BACKTRACE` env-var mutation carries an explicit, accurate `SAFETY` comment explaining exactly why the `unsafe` env write is sound at this point in boot (no other threads reading/writing env yet) — a good example of handling Rust's edition-gated unsafe-env-var API responsibly rather than just silencing it.
- `environment.rs` as a whole — 138 lines doing exactly one job (resolve environment name → load config), with a graceful fallback (`Environment::Any(String)`) instead of a hard error for unrecognized environment names, and three focused tests.

## Top 3 things that would most raise the area's quality

1. **Fix the `on_shutdown` lifecycle gap** (Evidence #1): route worker-only shutdown through the same `on_shutdown` call the HTTP path uses (e.g. call `H::on_shutdown(&app_context)` in `boot.rs:140-145` before/after `shutdown_and_await_queue_worker`), and add a test that boots `StartMode::WorkerOnly`/`WorkerAndScheduler` and asserts the hook fires — closing both the contract bug and the zero-coverage gap in one move.
2. **Add unit tests for `boot.rs`** — today it has none. At minimum: one test per `StartMode` arm of `run_app` (asserting the resulting `BootResult` shape), a `create_context` test asserting config-driven mailer/queue/cache wiring, and a `run_db` test per `RunDbCommand` arm against `tests_cfg`'s dummy DB.
3. **Collapse the `run_app` `StartMode` match and the `create_app` with/without-db split** into single functions parameterized by 2-3 booleans (`needs_router`, `needs_workers`, `needs_scheduler`) computed once from `mode`, and fold `extra_db.rs`/`multi_db.rs` into one generic `Initializer` helper — removing ~100 combined lines of duplicated control flow with no loss of clarity.
