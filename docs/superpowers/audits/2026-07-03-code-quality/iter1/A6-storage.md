# Area: A6 · Storage

## Scope (files reviewed, with LOC)

- `src/storage/mod.rs` (457) — `Storage` façade, `StorageError`
- `src/storage/stream.rs` (80) — `BytesStream` (axum/opendal bridge)
- `src/storage/contents.rs` (38) — `Contents` newtype (orphan-rule workaround for `TryFrom`)
- `src/storage/strategies/mod.rs` (33) — `StorageStrategy` trait
- `src/storage/strategies/single.rs` (237, incl. ~120 lines tests)
- `src/storage/strategies/mirror.rs` (812, incl. 486 lines tests, lines 327-812)
- `src/storage/strategies/backup.rs` (1244, incl. 960 lines tests, lines 285-1244)
- `src/storage/drivers/mod.rs` (152) — `StoreDriver` trait, `GetResponse`/`UploadResponse`
- `src/storage/drivers/opendal_adapter.rs` (185) — the one real driver, wraps `opendal::Operator`
- `src/storage/drivers/aws.rs` (120), `azure.rs` (32), `gcp.rs` (23), `local.rs` (41), `mem.rs` (24), `null.rs` (95)

Total 3,573 LOC as specified. All files read in full.

## Scores

| KPI | Score | One-line justification w/ primary cite |
|---|---|---|
| 1. Holistic vision | 5 | `mirror.rs` itself is internally inconsistent: `upload`/`delete` check `should_fail` once after the secondary loop (mirror.rs:69-87, 122-144) but `rename`/`copy` check it **inside** the loop on every iteration (mirror.rs:160-177, 191-208) — same file, two different unwritten policies for "the same problem." |
| 2. Economy of concepts | 4 | Two full strategy structs + two near-duplicate `FailureMode` enums (backup.rs:32-43 vs mirror.rs:31-36) implement the same "primary + fan-out-to-secondaries + collect errors" concept twice end-to-end, differing only in the download fallback behavior and the failure-threshold vocabulary. |
| 3. Low LOC | 3 | backup.rs's 4 mutating methods (upload/delete/rename/copy, backup.rs:61-196) are the same ~20-line error-collection loop copy-pasted 4×, then that whole file is copy-pasted again (with 2 tweaks) as mirror.rs. Tests add another ~1,450 lines of copy-pasted 3-store-BTreeMap boilerplate (backup.rs:285-1244, mirror.rs:326-812). |
| 4. Non-brittle | 3 | Silent-wrong `exists()`: opendal_adapter.rs:143-146 swallows *any* opendal error (permission, network, timeout) into `Ok(false)`, acknowledged by its own TODO (opendal_adapter.rs:138-142). Also the mirror.rs rename/copy early-return-in-loop bug above silently leaves secondaries un-mirrored without any error being surfaced for the skipped ones. |
| 5. Maintainable (DDD/OOP) | 5 | Domain concepts (`StoreDriver`, `StorageStrategy`, `FailureMode`) are well-named and the split of driver vs. strategy is sound, but duplicating the entire fan-out algorithm per strategy means every future bugfix (see mirror.rs bug) must be manually re-applied in two places — a maintainability trap already manifesting. |
| 6. Correctness | 4 | `download_stream`/`upload_stream` (added in #1610, backup.rs:203-258, mirror.rs:220-291) have **zero** tests in backup.rs/mirror.rs's test modules (checked: no `download_stream`/`upload_stream`/`stream` test names present, grep confirms no test dir references either). The mirror.rs rename/copy bug (finding 2) is also untested — no test asserts the state of the *second* secondary after a first-secondary failure. |
| 7. No reinvented wheels | 8 | The driver layer is a genuinely thin, disciplined wrapper: `aws.rs`/`azure.rs`/`gcp.rs`/`local.rs`/`mem.rs` are each 1-3 constructor functions (~20-40 lines) that build an `opendal::services::*` config and hand it to `OpendalAdapter::new` (e.g. gcp.rs:17-23). All actual I/O logic lives in one place, opendal_adapter.rs, which correctly defers to OpenDAL's own capability negotiation (`full_capability().rename`/`.copy`, opendal_adapter.rs:83,106) instead of reinventing it. |
| **Overall** | **5** | Driver layer is excellent and exactly as thin as it should be; the strategies layer is the known suspect confirmed — real doc'd behavior (backup vs. mirror, 4 failure modes) but delivered via wholesale copy-paste of an algorithm that should be a single parameterized helper, with a live correctness bug that duplication let slip through unnoticed. |

## Evidence log

1. **FACT**: mirror.rs `upload` (mirror.rs:63-90) and `delete` (mirror.rs:122-145) call `self.failure_mode.should_fail(&collect_errors)` once, *after* the `for secondary_store in secondaries` loop completes. mirror.rs `rename` (mirror.rs:154-181) and `copy` (mirror.rs:188-212) call the same check *inside* the loop body, per secondary, with an early `return Err(...)`.
   **Judgment**: Under `FailureMode::MirrorAll` with 2+ secondaries, if the *first* secondary's rename/copy fails, the method returns immediately and **never attempts the remaining secondaries** — they stay at the old path/未copied, permanently diverging from the primary, with no error reported about them. `upload`/`delete` do not have this bug because they finish the loop first.
   **KPI(s)**: 1 (holistic vision), 4 (non-brittle), 6 (correctness). **Severity: HIGH** — silent partial-mirror state in a strategy whose entire purpose is keeping backends in sync.

2. **FACT**: No test in mirror.rs's `#[cfg(test)] mod tests` (mirror.rs:326-812) asserts the state of a *third* store when a fan-out of 2+ secondaries is used and the *first* secondary fails during rename/copy. `rename_should_fail_when_primary_failed` (mirror.rs:571-613) only asserts `is_err()`, never checking `store_3`'s existence at either path.
   **Judgment**: The exact scenario that would expose finding 1 is not covered — test coverage of failure paths (KPI6, flagged as critical in the task) has a real gap here.
   **KPI(s)**: 6. **Severity: HIGH**.

3. **FACT**: `opendal_adapter.rs:143-146` — `async fn exists(&self, path: &Path) -> StorageResult<bool> { ... Ok(self.opendal_impl.exists(&path).await.unwrap_or(false)) }`, with its own doc comment (opendal_adapter.rs:138-142) admitting: "The `exists` function should return an error for issues such as permission denied. However, these errors are not handled... and should be addressed after the test suites are refactored."
   **Judgment**: This is a self-documented silent-wrong: any transport/permission error is indistinguishable from "file does not exist" to every caller of `StoreDriver::exists`, including any future strategy logic that branches on existence.
   **KPI(s)**: 4, 6. **Severity: MEDIUM-HIGH** (acknowledged tech debt left unresolved across at least the last several releases — patch-on-patch smell).

4. **FACT**: backup.rs's 4 mutating trait methods (`upload` 61-88, `delete` 103-127, `rename` 136-163, `copy` 172-196) are structurally identical: call primary, loop `secondaries` building a `BTreeMap<String,String>` of stringified errors via the same `match storage.as_store_err(...) { Ok(store) => if let Err(err) = store.OP(...).await { insert } , Err(err) => insert }` pattern, then check `failure_mode.should_fail`. The same 4-method pattern is repeated in mirror.rs (63-90, 122-145, 154-181, 188-212) with only the `FailureMode` type and (for mirror) the extra "check inside loop" quirk differing.
   **Judgment**: This is exactly the KNOWN SUSPECT confirmed — ~8 near-identical implementations of one "fan-out-with-error-collection" algorithm exist where a single generic helper (e.g. `fan_out<F>(storage, secondaries, op: F) -> BTreeMap<String,String>`) parameterized by an async closure would collapse backup.rs+mirror.rs from ~2,056 LOC to an estimated ~500-700 LOC including tests.
   **KPI(s)**: 2, 3, 5. **Severity: MEDIUM** (not a bug, but the direct cause of finding 1 slipping through).

5. **FACT**: `FailureMode::should_fail` is duplicated as two separate impls: backup.rs:274-282 (`BackupAll`/`AllowBackupFailure`/`AtLeastOneFailure`/`CountFailure(usize)`) and mirror.rs:316-323 (`MirrorAll`/`AllowMirrorFailure`, a strict subset of backup's semantics — `MirrorAll` ≡ `BackupAll`, `AllowMirrorFailure` ≡ `AllowBackupFailure`).
   **Judgment**: Mirror's `FailureMode` could be `backup::FailureMode` (or a shared enum) rather than a hand-duplicated 2-variant clone; this is concept sprawl for zero behavioral gain (mirror never needs `AtLeastOneFailure`/`CountFailure`, but nothing prevents reusing the richer enum).
   **KPI(s)**: 2, 3. **Severity: LOW-MEDIUM**.

6. **FACT**: `mod.rs:369` contains an un-resolved self-review comment: `// REVIEW(nd): not sure bout the name 'as_store_err' -- it returns result`.
   **Judgment**: A live "note to self" left in shipped code — small but a textbook patch-on-patch smell per the rubric's list.
   **KPI(s)**: 1. **Severity: LOW**.

7. **FACT**: `StorageError::Multi(BTreeMap<String, String>)` (mod.rs:38-39) stores secondary-store errors as stringified text (`err.to_string()`, e.g. mirror.rs:75, backup.rs:73), discarding the original `StorageError`/`opendal::Error` type and any downcast-ability.
   **Judgment**: Minor stringly-typed error handling; acceptable for a BTreeMap-of-diagnostics but forecloses programmatic handling of *which kind* of failure occurred per secondary.
   **KPI(s)**: 4. **Severity: LOW**.

## Patch-on-patch smells (specific, cited)

- `src/storage/mod.rs:369` — `REVIEW(nd)` comment left in shipped code, unresolved naming doubt about `as_store_err`.
- `src/storage/drivers/opendal_adapter.rs:138-142` — TODO admitting `exists()` swallows real errors as `false`, "should be addressed after the test suites are refactored" (still unaddressed).
- `src/storage/drivers/mod.rs:26-29` — TODO: "Add more methods to `GetResponse` to read the content in different ways... e.g. read a specific range of bytes" — an acknowledged missing capability (range reads), not implemented.
- `src/storage/strategies/mirror.rs` vs `src/storage/strategies/backup.rs` — whole-file duplication (see Evidence #4/#5); the divergent should_fail-placement bug in mirror.rs (Evidence #1) is a direct symptom of copy-paste evolution rather than a shared abstraction.
- `src/storage/drivers/opendal_adapter.rs:107-108` — comment "opendal 0.57's `copy` returns the destination `Metadata`; we don't surface it" — a version-drift shim note; harmless but marks an API surface that's silently dropping data OpenDAL now provides.

## Library hypotheses

1. **HYPOTHESIS**: Replace the hand-rolled `BTreeMap<String,String>` "fan-out and collect per-secondary errors" pattern (repeated 8× across backup.rs/mirror.rs) with `futures::future::join_all` / `futures_util::stream::FuturesUnordered` to run secondary operations concurrently while a small shared helper does the collection.
   - Why it MIGHT be simpler: `futures_util` is already a direct dependency (used in stream.rs:5, opendal_adapter.rs:5); this only changes *how* the loop is structured, not what's imported — reduces the 8 duplicate loops to 1 generic function, and turns today's serial secondary writes into concurrent ones (latency win for real backup/mirror use).
   - Risk / why it might NOT fit: Concurrent writes change failure semantics subtly (order-dependent `CountFailure`/`AtLeastOneFailure` interpretations may need re-specification because errors could interleave); also `FailureMode` variance (2 vs 4 modes) still needs unifying by hand first.
   - **NEEDS SPIKE**.

2. **HYPOTHESIS**: Unify `backup::FailureMode` and `mirror::FailureMode` into one enum in `strategies/mod.rs`, with `MirrorStrategy` using a subset. This isn't a crate swap, just a hand-rolled-vs-hand-rolled simplification, but worth flagging since it's the direct enabler of collapsing the two strategy files into one generic `FanoutStrategy<Policy>`-style implementation.
   - Why it MIGHT be simpler: single source of truth for "how do we decide if the multi-store operation failed," used by both strategies.
   - Risk: `BackupStrategy` and `MirrorStrategy` are public API (`pub struct`, `pub enum FailureMode` in each module) — this is a breaking rename/move for downstream users already on 0.17.0's Sea-ORM-driven breaking cycle, so it should ride along with that breaking-change window rather than ship standalone.
   - **NEEDS SPIKE**.

3. No credible crate replacement identified for `opendal_adapter.rs` itself — it already *is* the OpenDAL usage; KPI7 for the driver layer is high (8/10) precisely because it does not reinvent OpenDAL's retry/capability/rename/copy logic (opendal_adapter.rs:19-21 RetryLayer, :83 `full_capability().rename`, :106 `full_capability().copy`).

## What is genuinely excellent (cited — be specific)

- **Driver layer discipline**: `aws.rs`, `azure.rs`, `gcp.rs`, `local.rs`, `mem.rs` are each nothing but a constructor building an OpenDAL `services::*` builder and wrapping it in `OpendalAdapter::new` — e.g. the entirety of `azure.rs:17-32` and `gcp.rs:17-23`. No hand-rolled HTTP, auth, or retry logic anywhere in the driver files; all delegated to OpenDAL. This is a model example of a thin adapter layer.
- **Capability-aware fallback in `opendal_adapter.rs:82-129`**: `rename`/`copy` check `opendal_impl.info().full_capability()` and only fall back to manual copy+delete / read+write-stream when the backend genuinely lacks native support — correct behavior expressed with minimal code, not reinvented.
- **`NullStorage` (drivers/null.rs)**: a clean, minimal default driver so Loco boots without requiring a storage feature flag decision up front (null.rs:1-6 docstring explains the rationale clearly) — every method is one line returning a clear `StorageError::Any("Operation not supported by null storage")`.
- **`stream.rs`**: `BytesStream` cleanly hides `opendal::Reader`/`opendal::Error` from the public API surface (stream.rs:8-9 doc comment) and provides both a `Stream` impl and an `into_body()`/`from_body_stream()` axum bridge in ~80 lines — tight and purposeful.
- **`Contents` newtype (contents.rs)**: correctly exists only to route around Rust's orphan rule (can't `impl TryFrom<Bytes> for String` directly since neither type is local) — a justified, minimal 38-line wrapper, not over-engineering.

## Top 3 things that would most raise the area's quality

1. Fix the mirror.rs rename/copy early-return-in-loop bug (Evidence #1) and add a test that asserts the *un-failed* secondary's state after a first-secondary failure under `MirrorAll` — this is a live correctness bug in a "keep backends in sync" feature.
2. Collapse backup.rs + mirror.rs's duplicated fan-out algorithm into one shared helper (possibly using `FuturesUnordered` for concurrency) parameterized by a unified `FailureMode`, cutting ~2,056 LOC to an estimated 500-700 LOC and eliminating the class of bug in #1 by construction.
3. Add test coverage for `download_stream`/`upload_stream` in both `BackupStrategy` and `MirrorStrategy` (currently absent), and resolve or remove the `exists()` silent-error-swallowing TODO in opendal_adapter.rs:138-146.
