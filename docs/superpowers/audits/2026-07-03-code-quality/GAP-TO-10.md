# What Keeps Loco From 10 — Gap Register + Governor Decisions

**Date:** 2026-07-04. Derived from the post-remediation 8-KPI scorecard (mean 6.67) by
mining every scorer's `delta_rationale` / `k8_rationale` for the concrete thing that caps
each cell, then deciding per blocker whether it is solvable.

**Meta-verdict up front:** a straight-10 board is **not attainable, and is the wrong goal.**
Roughly a third of the residual gap is *by-design* (K7) or needs an *upstream* change outside
this repo (axum). The other two-thirds — test coverage of shell/wiring code, killing
sleep-based test waits, and finishing the internal DRY — is fully within reach and would land
the trough areas (A1/A3/A9) at 8–9 and the overall mean near 8. **That** is the real ceiling.

---

## Bucket A — CAN solve, and worth doing (governor: YES)

### A-i. Sleep-based test waits → readiness polling *(highest leverage — one root fix lifts K8 in 5+ areas)*
The single most-cited K8 drag. `tests/infra_cfg/server.rs:43` `start_from_boot` does a hardcoded
`sleep(2s)` before returning — **every** integration test in A1/A3/A4/A8 inherits it (flaky under
CI load, slow ×20+ cases). Plus point-sleeps: worker-loop `sleep(2s)` in `redis.rs:1358` /
`pg.rs:1142` / `sqlt.rs:1367` (A5); `jwt.rs:177` `sleep(3s)` for expiry (A8); `engine.rs:305`
`sleep(300ms)` file-watcher (A4); redis cache `test_expiry` `sleep(2s)` and `scheduler::can_run`
`sleep(5s)` (A11).
**Decision: SOLVABLE.** Replace the shared helper's fixed sleep with a TCP/health-endpoint retry
poll; use `0s TTL + leeway` for expiry tests. Do this **first** — it's self-contained and lifts
K8 across A1, A3, A4, A5, A8, A11 at once.

### A-ii. Untested shell/wiring code → author the missing tests *(the K6 drag — original audit theme)*
- `boot.rs` has **no `#[cfg(test)]` block at all**: `start()` mode dispatch, `run_app`,
  `register_workers`, scheduler wiring untested (A1). Critically, the shipped `#3 on_shutdown`
  fix has **zero** test — nothing boots `WorkerOnly`/`WorkerAndScheduler` and asserts the hook
  fires, so it can silently regress. (Needs an injectable shutdown signal to test cleanly.)
- `cli.rs` — no test touches `dispatch_common` / the twin mains / the create_context fix (A10).
- `logger.rs` (225 lines of branchy EnvFilter precedence + file-appender wiring) — 0 tests (A9/A3).
- Reaper *loop* behavior (spawn + `select!`) untested — only config→RunOpts wiring is (A5).
- `engine_embedded.rs` render path — 0 runtime tests (A4). Middleware `compression`/`format`/
  `static_assets_embedded` — 0 tests (A3).
**Decision: SOLVABLE.** Straight test authoring; `on_shutdown` needs a small seam
(inject `CancellationToken`) but is doable. Lifts K6 in A1, A3, A5, A9, A10 + K8 everywhere.

### A-iii. Residual duplication not yet collapsed *(K3 drag)*
- `backup.rs` still carries the 5×-duplicated fan-out loop that `mirror.rs` shed — same shape,
  untouched because its loop was already non-short-circuiting (A6).
- `Queue::run` triplicates the reaper-spawn boilerplate per backend, `mod.rs:225-293` (A5).
- `register_worker` panic-wrapper closure duplicated `sql.rs` vs `redis.rs` (A5).
- StartMode resolution (~15 lines) still copy-pasted in both cli mains, `cli.rs:730-747` vs
  `904-921` (A10).
**Decision: SOLVABLE.** Extract a strategy-level fan-out helper (backup), factor the reaper
spawn once, hoist the panic-wrapper, move StartMode resolution into `dispatch_common`.

### A-iv. Latent footguns left in place *(K4 drag)*
- `engine.rs:182,197` `lock().unwrap()` → panics on mutex poisoning (A4). Map to error.
- `cli.rs:878` the new `_ => unreachable!()` I introduced via `dispatch_common` trades
  compile-time exhaustiveness for a runtime panic on future `Commands` variants (A10). Restore an
  explicit exhaustive match.
- `environment.rs:103-119` test mutates real env via `unsafe set_var` with no `serial_test` —
  parallel-test flakiness (A9).
- Postgres tests hard-fail with a raw `SocketNotFoundError` when Docker is absent, no skip-gate
  (A12) — `loco-gen/tests/db.rs` already models the graceful skip.
- `Cargo.toml` still enables moka's now-dead `sync` feature (A11); stale `// XXX fix` comments at
  `schema.rs:792/805/817` (A7).
**Decision: SOLVABLE.** All low-risk housekeeping; lifts K4 in A4/A9/A10/A11 + cleans A7/A12.

---

## Bucket B — CAN solve, but governor DECLINES (churn > value)

- **Twin `create_app` (with-db vs not)** — a 3-line, feature-gated diff; both arms call the same
  `run_app`/`create_context`. Unifying behind an optional-migrator closure adds an abstraction
  layer for duplication that is small and drift-resistant. **DECLINE.**
- **Collapse the 6 validate marker structs further** — the *real* duplication (per-extractor
  decode+validate+map) is already gone via 3 decoder fns + 2 macros. What remains is per-extractor
  marker/doc; collapsing to one generic struct+param trades legibility for fewer lines. **DECLINE.**
- **Split `errors.rs`'s flat `Error` enum** (HTTP-domain vs infra concerns) — a large, invasive
  refactor touching every `?` site and `IntoResponse`; real regression risk for a marginal K5 gain.
  **DECLINE now**; revisit only if the enum keeps growing.

---

## Bucket C — CANNOT cleanly reach 10 (inherent ceilings — by-design or upstream)

- **K7 No-reinvention (mean 6.5).** The audit validated **7 rejected library swaps** as Loco's
  deliberate product value (hand-rolled queue, remote-IP/XFF handling, number formatting,
  two-tier validation errors). A "10" here would mean adopting crates that *don't fit* and
  regressing capability. **Ceiling is by design (~7–9). NOT solvable without loss.**
- **`describe.rs` verb extraction (caps A2 K4=7/K7=5).** It regexes over axum's *private* Debug
  format because axum exposes no public per-verb routing API. A canary test already guards the
  assumption; a true fix requires an **upstream axum change** (I can file a PR there — outside this
  repo). **Structural ceiling, not in Loco's sole control.**
- **Hand-rolled polling workers over sqlx/redis (caps A5 K7=5).** apalis et al. were rejected —
  can't express the runtime string-keyed registry, priority, tags, or admin ops. The reinvention
  *is* the feature. **NOT solvable without loss.**
- **Real cron-fire timing in `scheduler::can_run` (A11).** A genuine cron tick needs wall-clock
  unless `tokio_cron_scheduler` exposes a mockable clock (it doesn't cleanly). Can shorten/poll,
  but some irreducible timing remains. **Minor, partial.**

---

## Recommendation

Execute **Bucket A** as one more governed pass (Sonnet authors, Opus verifies + commits,
sequentially), ordered by leverage: **(1)** readiness-poll → **(2)** shell coverage →
**(3)** residual DRY → **(4)** footgun cleanup. Projected result: K4→~8, K6→~8, K8→~8,
trough areas A1/A3/A9 to 8, **overall mean ~7.8–8.0**. Decline Bucket B. Accept Bucket C as
**documented, defensible ceilings** — the honest answer to "why not 10" for K7 and `describe.rs`.
