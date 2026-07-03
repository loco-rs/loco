# Area: A11 · Services: Mailer, Cache, Scheduler, Tasks

## Scope (files reviewed, with LOC)

- `src/mailer/mod.rs` (172)
- `src/mailer/template.rs` (98)
- `src/mailer/email_sender.rs` (292)
  — mailer subtotal 562 (matches AREAS.md)
- `src/cache/mod.rs` (520)
- `src/cache/drivers/mod.rs` (80)
- `src/cache/drivers/null.rs` (110)
- `src/cache/drivers/inmem.rs` (234)
- `src/cache/drivers/redis.rs` (299)
  — cache subtotal 1243 (matches AREAS.md)
- `src/scheduler.rs` (628/629)
- `src/task.rs` (279)

All files read in full.

## Scores

| KPI | Score | One-line justification w/ primary cite |
|---|---|---|
| 1. Holistic vision | 6 | Each subsystem is internally clean, but the *same* concern — "pick a backend implementation from config" — is solved with a trait-object driver for cache (`src/cache/mod.rs:45-62`, `Box<dyn CacheDriver>`) vs. a closed enum matched inline for mailer (`src/mailer/email_sender.rs:17-23,162-170`); plus a leftover TODO (`src/mailer/template.rs:68`) and a "previously duct_sh" migration comment (`src/scheduler.rs:197-201`) show visible patch history. |
| 2. Economy of concepts | 8 | Small, well-bounded type sets per subsystem: `Mailer`/`EmailSender`/`Template` (mailer), `CacheDriver`+3 drivers (cache), `Config`/`Job`/`Scheduler`/`JobDescription` (scheduler), `Task`/`Tasks` (task) — no sprawl, e.g. `src/task.rs:75-124` is the entire registry concept. |
| 3. Low LOC | 6 | `src/cache/mod.rs` is 520 lines but ~300 of them are repetitive doctest boilerplate re-declaring `InMemCacheConfig { max_capacity: 100 }` per method (e.g. `src/cache/mod.rs:84-86,104-106,131-133,178-180,226-228`); `src/cache/drivers/null.rs:36-38,47-49,66-69,78-87,94-97,105-108` is the same one-line error body copy-pasted 6×. |
| 4. Non-brittle | 6 | `CacheError::Any(Box<dyn Error>)` is a stringly-typed catch-all used for "not supported" (`src/cache/drivers/null.rs:36-38` etc.); scheduler's cron-vs-English detection is a single hand-rolled regex `^[\*\d]` (`src/scheduler.rs:19-23,306-315`) with no defensive trim/validation of leading whitespace. |
| 5. Maintainable | 7 | Clear, well-named domain boundaries (mailer/cache/scheduler/task each own their model); `Job::prepare_command`/`JobDescription::run` (`src/scheduler.rs:158-211`) cleanly separates "build command" from "execute command". |
| 6. Correctness | 7 | Good, real tests: mailer TLS-mode matrix (`src/mailer/email_sender.rs:184-213`), scheduler end-to-end cron tick test (`src/scheduler.rs:520-627`), task registry override semantics explicitly tested (`src/task.rs:247-278`). Gap: `src/cache/drivers/null.rs` has **no** `#[cfg(test)]` module at all despite being the framework's default driver (doc at `null.rs:3-6`). |
| 7. No reinvented wheels | 9 | Scheduler is a thin wrapper over `tokio_cron_scheduler` (`src/scheduler.rs:14,300-349`) and `english_to_cron` (`src/scheduler.rs:309`), not a hand-rolled cron engine — AREAS.md's reinvention suspicion is **not borne out**. Cache directly wraps `moka` (`src/cache/drivers/inmem.rs:10`) and `bb8_redis` (`src/cache/drivers/redis.rs:7-12`). Mailer wraps `lettre` (`src/mailer/email_sender.rs:5-9`). |
| **Overall** | **7** | Four genuinely thin, well-tested library wrappers; the score is held back by cross-subsystem inconsistency in the "driver selection" idiom, some stringly-typed error handling in cache, and a couple of untested/undertested paths (null driver). |

## Evidence log

1. FACT: `src/mailer/template.rs:68` contains `// TODO(consider): check+consider offloading to tokio async this work` — live TODO in shipped code. JUDGMENT: patch-on-patch smell, unresolved design question left in the tree. KPI: 1 (holistic vision). SEVERITY: low.
2. FACT: `src/cache/drivers/null.rs:36-38, 47-49, 66-69, 78-87, 94-97, 105-108` each independently construct `Err(CacheError::Any("Operation not supported by null cache".into()))`. JUDGMENT: 6-way copy-paste of an identical error, no shared helper/const; also `CacheError::Any` is an untyped `Box<dyn Error>` wrapping a plain string, so callers can't match on this case structurally. KPI: 3 (Low LOC), 4 (non-brittle). SEVERITY: medium.
3. FACT: `src/cache/drivers/null.rs` has no `#[cfg(test)] mod tests` (confirmed by full read — file ends at line 110 with the trait impl, no test module), while `inmem.rs:168-234` and `redis.rs:145-299` both have thorough driver test suites. JUDGMENT: the framework's *default* cache driver (per its own doc comment, `null.rs:3-6`) ships with zero unit tests exercising its actually-non-erroring `get` path (`null.rs:57-59`, which is `Ok(None)` while every other method errors) — an inconsistency within the driver itself that no test catches. KPI: 6 (correctness), 4 (non-brittle). SEVERITY: medium-high.
4. FACT: Cache picks a backend via `Box<dyn CacheDriver>` selected in `create_cache_provider` (`src/cache/mod.rs:45-62`); Mailer picks a backend via a closed `enum EmailTransport { Smtp(...), Test(...) }` matched with `match &self.transport` (`src/mailer/email_sender.rs:17-23, 162-170`). JUDGMENT: same architectural problem ("select at runtime among a small set of backend implementations") solved with two different idioms in sibling subsystems of the same area — a clear KPI1 "inconsistent approach to the same problem" signal the task asked to look for. KPI: 1 (holistic vision), 2 (economy of concepts — two idioms doing one job). SEVERITY: medium.
5. FACT: `src/cache/drivers/inmem.rs:10,33-34,50-58` builds and drives a `moka::sync::Cache` (blocking API) from behind the `#[async_trait]` `CacheDriver` trait, while `redis.rs` behind the same trait is genuinely async end-to-end (`bb8`/`bb8_redis`, `redis.rs:62-142`). JUDGMENT: works today because moka's sync ops are in-memory and fast, but it's an inconsistent "async-in-name-only" implementation of the same trait one file over from a truly-async sibling; `moka::future::Cache` exists and would remove the mismatch. KPI: 1 (holistic vision), 4 (non-brittle). SEVERITY: low.
6. FACT: `src/scheduler.rs:19-23` uses a hand-written regex `^[\*\d]` to decide whether `job.cron` (`scheduler.rs:306-315`) is raw cron syntax or an English phrase to hand to `english_to_cron`. JUDGMENT: no `.trim()` before the check, and the heuristic is a single-character sniff rather than an actual cron-grammar check; a config value with leading whitespace or a non-numeric-first-field cron dialect would silently misroute to the English parser and surface as a possibly-confusing `InvalidCronSyntax` rather than being handled directly. It does fail loudly (not silently-wrong), so this is a minor robustness gap, not a correctness bug. KPI: 4 (non-brittle). SEVERITY: low.
7. FACT: `src/scheduler.rs:197-201` comment reads "Run the command through the platform shell (previously `duct_sh`)" — a migration note left in the source. JUDGMENT: harmless but is literally the kind of "version-drift shim / patch history" comment the rubric calls out to hunt for. KPI: 1. SEVERITY: low.
8. FACT: `src/task.rs:119-123` `Tasks::register` silently overwrites any existing task with the same name (`self.registry.insert(name, Box::new(task))` with no check/log). JUDGMENT: this is deliberate and *is* tested (`task.rs:247-278`, `test_task_registration_and_override`), so it's a documented design choice, not a bug — flagged only because a silent override with no `tracing::warn!` could surprise a user who defines two tasks with the same name by accident (e.g. copy-pasted task file). KPI: 4 (non-brittle). SEVERITY: low.

## Patch-on-patch smells

- `src/mailer/template.rs:68` — TODO comment shipped in the tree.
- `src/scheduler.rs:198` — comment referencing a prior implementation (`duct_sh`) that was migrated away from.
- `src/mailer/email_sender.rs:196-198` — test comment explicitly documents a previously-shipped bug ("the case that was previously impossible: `secure: true` only ever did STARTTLS") that was since fixed — good that it's fixed and tested, but confirms mailer TLS handling evolved through at least one incorrect iteration before reaching current state.
- `src/cache/drivers/null.rs:36-108` — six structurally identical error-construction blocks, the copy-paste signature of a driver grown method-by-method rather than designed as one shape (e.g. via a `fn unsupported<T>() -> CacheResult<T>` helper).
- Two idioms for "select backend impl from config" across sibling subsystems (cache: trait object; mailer: closed enum) — see Evidence #4. Not literally dead/duplicated code, but the kind of area-wide inconsistency the rubric singles out as a holistic-vision smell.

## Library hypotheses

1. Hand-rolled: none for scheduler's core cron execution — `src/scheduler.rs` already delegates to `tokio_cron_scheduler` (`scheduler.rs:14,300-349`) and `english_to_cron` (`scheduler.rs:309`) for the hard parts. **No swap hypothesis here** — AREAS.md's suspicion that this reinvents `tokio-cron-scheduler`/`cron` is not supported by the code; it's already a thin wrapper (config file → task-name validation → per-job `tokio_cron_scheduler::Job` registration → subprocess dispatch via `duct`). Flagging explicitly so the orchestrator doesn't re-flag it.
2. Hand-rolled: `src/cache/drivers/inmem.rs` wraps `moka::sync::Cache` inside an async trait (`inmem.rs:10,33-58`). HYPOTHESIS: switch to `moka::future::Cache` so the "async" `CacheDriver` implementation for in-mem is actually async end-to-end, matching the Redis driver's real-async shape. WHY IT MIGHT BE SIMPLER: `moka::future::Cache` has a near-identical API (`get`/`insert`/`invalidate_all`), so the diff would be small. RISK/WHY IT MIGHT NOT FIT: current sync usage is not actually broken (moka sync ops don't block meaningfully), so this is cosmetic/consistency-only, not fixing a real defect — low priority. NEEDS SPIKE.
3. Hand-rolled: `src/cache/drivers/null.rs:36-108` repeated "operation not supported" error construction. HYPOTHESIS: not really a "library" fix — a local `fn unsupported<T>() -> CacheResult<T>` helper (no external crate) would collapse 6 duplicated blocks to 1. Mentioning here because the fix is cheap and the rubric explicitly asks to flag copy-paste. NEEDS SPIKE (trivial, in-house).

## What is genuinely excellent

- `src/mailer/email_sender.rs:46-86` — the TLS-mode handling (`Starttls`/`Implicit`/`None`) is clean, correctly distinguishes STARTTLS (587) from implicit TLS (465) from cleartext, and is paired with a real matrix test covering every mode plus the legacy `secure` flag's back-compat mapping (`email_sender.rs:184-213`). This is exactly the kind of "correct TLS handling" the task asked to verify, and it holds up.
- `src/scheduler.rs:520-627` (`can_run` test) — a genuine end-to-end integration test that runs the real `tokio_cron_scheduler`, waits 5 real seconds, and asserts on actual file-append side effects for 3 concurrently-scheduled jobs (including a `run_on_start` job) — this is real behavioral coverage, not a happy-path smoke test.
- `src/task.rs:75-124` — the `Task`/`Tasks` registry is about as small as this concept can be: a trait with 2 methods and a `BTreeMap`-backed registry with `register`/`run`/`list`/`names`. No indirection beyond what's needed.
- `src/cache/mod.rs:305-382` — `get_or_insert`/`get_or_insert_with_expiry` are a nice small "cache-aside" ergonomic addition, generic over any `Serialize + DeserializeOwned`, implemented in ~15 lines each, well tested (`cache/mod.rs:479-519`, verifies the closure is *not* re-invoked on hit).

## Top 3 things that would most raise the area's quality

1. Unify the "select backend implementation from config" idiom across mailer and cache (both toward trait objects, since cache's is more extensible and mailer's `Test` transport is already testing-only) — directly addresses the KPI1 holistic-vision signal this audit asked to look for.
2. Add a test module for `src/cache/drivers/null.rs` (trivial — assert every method errors except `get`, and document *why* `get` alone is `Ok(None)` instead of erroring) to close the correctness gap on the framework's default driver.
3. Replace the 6 duplicated "not supported" error blocks in `null.rs` with one helper, and consider giving `CacheError` a proper `Unsupported` variant instead of overloading `CacheError::Any(Box<dyn Error>)` for a case that isn't really "any error" — it's a fixed, known condition.
