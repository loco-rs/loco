# Loco Remediation — Execution Plan (from the 2026-07-03 audit)

**Governance:** Sonnet implements each task; Opus reviews/criticises/gates and commits locally.
**Constraint:** local commits only (branch `release/1.0.0`), atomic per-fix; user does all push/PR/publish.
**Method:** each task = fix + test (TDD where the bug is observable) + `cargo build`/targeted test green,
then Opus review before commit.

## In scope this pass (confirmed, bounded, test-backed)

| Task | Defects/Hyps | Files |
|------|--------------|-------|
| T1 Data layer | #2 dump_tables bool arm (+PG test), #7 `url` crate in connect, #8 reference_id normalization | `db/schema.rs`, `db/connect.rs`, `schema.rs` |
| T2 Errors/HTTP | #6 Error→HTTP mapping (EntityNotFound/Form/Storage/Cache→real status), #5 describe.rs multi-verb | `controller/mod.rs`, `errors.rs`, `controller/describe.rs` |
| T3 Boot/MW/Views | #3 on_shutdown in worker-only modes, #9 fallback default 404, #10 ViewEngine Infallible→real rejection | `boot.rs`, `app.rs`, `middleware/fallback.rs`, `controller/views/mod.rs` |
| T4 Storage | #1 mirror rename/copy fan-out fix + unify via `join_all` (H4) | `storage/strategies/mirror.rs`, `backup.rs` |
| T5 Config | #4 `.local.yaml` deep-merge (maps recurse, scalars/arrays replace) | `config/mod.rs` |
| T6 Bgworker | #11 remove fossil `sleep(3s)` on redis boot | `bgworker/redis.rs` |
| T7 Small wins + fossils | H6 `Cookie::value()`, H5 `moka::future`, #14 redaction regex, #15 JWT algorithm footgun, H11 doctor `semver` pre-release, delete `_archive/`+backtrace-comments+views-locale-block+xxx-fix comments | auth extractor, `cache/drivers/inmem.rs`, `testing/redaction.rs`, `auth/jwt.rs`, `doctor.rs`, misc |

## Deferred (with rationale — NOT this pass)

- **P2 architectural DRY** (redis→`Driver` trait; collapse 6 validate extractors + 2 JWT paths;
  twin `main`/`create_app`; format.rs doubled API). *Behavior-preserving refactors with real
  regression surface; belong in their own brainstorm→spec→plan cycle, not a hardening sweep.*
- **#12 visibility-timeout reaper.** *New feature (needs timeout config + per-backend design), not a bug fix.*
- **#13 test-db cleanup race.** *Subtle async-in-Drop change to test infra; low blast radius, defer to the DRY cycle.*

## Dependency changes (Opus applies centrally to avoid Cargo.toml merge churn)
- add `url = "2"` (transitive via sqlx → no new download) — T1
- moka features `["sync"]` → `["sync","future"]` — T7 (H5)
- doctor pre-release check: prefer already-present `reqwest` if it is a runtime dep; else add `ureq` — T7 (H11)
