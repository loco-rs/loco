# Loco Code-Quality Audit — Iteration 1 Consolidated Scorecard

12 areas reviewed by independent Sonnet architects against the 7-KPI rubric; every
finding cited to `file:line`; highest-stakes correctness claims re-verified by the
Opus governor against the source (noted ✓ below).

## Scores (1–10; 10 = best)

| Area | K1 Vision | K2 Economy | K3 LOC | K4 Brittle | K5 Maint | K6 Correct | K7 NoReinvent | **Overall** |
|------|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| A1 Boot & Lifecycle        | 7 | 7 | 6 | 4 | 7 | 4 | 7 | **6** |
| A2 HTTP Core & Routing     | 5 | 6 | 6 | 3 | 5 | 4 | 4 | **5** |
| A3 Middleware              | 6 | 6 | 5 | 6 | 7 | 4 | 5 | **6** |
| A4 Views & Templating      | 5 | 6 | 6 | 4 | 5 | 5 | 6 | **6** |
| A5 Background Workers       | 5 | 6 | 3 | 4 | 5 | 6 | 5 | **6** |
| A6 Storage                 | 5 | 4 | 3 | 3 | 5 | 4 | 8 | **5** |
| A7 Data Layer              | 5 | 4 | 5 | 3 | 6 | 4 | 5 | **5** |
| A8 Auth & Validation       | 6 | 5 | 6 | 5 | 7 | 7 | 7 | **7** |
| A9 Config & Observability  | 5 | 6 | 7 | 3 | 6 | 3 | 5 | **5** |
| A10 CLI & Codegen          | 6 | 6 | 4 | 5 | 6 | 6 | 7 | **6** |
| A11 Services               | 6 | 8 | 6 | 6 | 7 | 7 | 9 | **7** |
| A12 Testing                | 6 | 7 | 7 | 4 | 6 | 4 | 6 | **5** |
| **Mean**                   | **5.6** | **5.9** | **5.3** | **4.2** | **6.0** | **4.8** | **6.2** | **5.6** |

## The framework-wide profile (the story the numbers tell)

- **Strongest: K7 No-reinvented-wheels (6.2)** and **K5 Maintainable/DDD (6.0)**. Loco
  leans on good crates (lettre, moka, argon2, tokio_cron_scheduler, opendal, validator,
  Sea-ORM) and models clean domain boundaries. Reviewers *debunked* three reinvention
  suspicions with evidence (scheduler, config-vs-figment, fixture-duplication).
- **Weakest: K4 Non-brittle (4.2)** and **K6 Correctness/test-coverage (4.8)**. This is
  precisely the "patch-on-patch evolution" signature: the architecture is sound, but
  edge-cases, silent-wrong paths, and untested "shell" files accumulated.

## The two dominant cross-cutting themes

**1. Duplication that has drifted out of sync (the patch-on-patch fingerprint).**
Nearly every area has a copy-paste pair where the copies diverged and a bug slipped into
the divergence:
- Storage `mirror.rs`: `upload`/`delete` fan out to all secondaries; `rename`/`copy`
  short-circuit mid-loop (`mirror.rs:174-176,205-207`) → silent un-mirrored secondaries. ✓verified
- Bgworker: `redis.rs:36-236` re-derives the Job/Registry/poll-loop that `sql.rs` already
  generalizes; `pg.rs`/`sqlt.rs` `enqueue`/`to_job`/`fail_job` are 85-100% identical.
- HTTP `format.rs`: six response concepts implemented twice (free fns vs `RenderBuilder`),
  already diverged (`json()` uses `axum::Json`; `RenderBuilder::json` hand-writes bytes).
- Boot: twin `create_app` (`boot.rs:403-437`); CLI: twin `main` (`cli.rs:712-869` vs `:872-1015`).
- Auth: two JWT validate-extract paths (`auth.rs:67-99` vs `:126-146`); 6 near-identical
  validate extractors. Config: `env_vars.rs` constants re-duplicated in `environment.rs`.

**2. Untested "shell" code + latent correctness bugs.** The core abstractions are tested;
the outer wiring often is not, and that is where the bugs live:
- `on_shutdown` never fires in `WorkerOnly`/`WorkerAndScheduler` (`boot.rs:140-146` bypasses
  `H::serve`). ✓verified
- Postgres `BOOLEAN` silently dropped from `dump_tables` (no `bool` arm, SQLite-only test). ✓verified
- `Config::from_folder` first-file-wins, no merge — `.local.yaml` must restate everything. ✓verified
- `fallback` default status is 200 despite docs/comments promising 404. ✓verified
- `describe.rs:22` first-match-only regex drops all-but-first HTTP verb from `cargo loco routes`
  (reviewer reproduced standalone vs axum 0.8.9).
- `redaction.rs:15` password regex `(.*{60})` is inert (≡ `.*`), zero tests (reviewer compiled it).
- Postgres admin-URL built via whole-string `.replace(db_name,"/postgres")` corrupts when a
  role name contains the db name (`connect.rs:173`). ✓verified (Med-High)
- `ViewEngine` extractor declares `Rejection=Infallible` but `.expect()`-panics (`views/mod.rs:74,82`).

## Confirmed dead/fossil code in the shipped tree
- `middleware/_archive/content_etag.rs` (103 LOC, no `mod` decl) ✓verified
- `backtrace.rs:18-35` 17 commented-out regex lines
- `views/mod.rs:83-92` abandoned locale block w/ `// BUG:` self-comment
- `schema.rs:792,805,817` `// xxx fix` / `// XXX fix` migration-DSL patch comments
- `JWT::algorithm()` dead-end builder (asymmetric algs can't work; hardcoded base64 secret)

## Highest / lowest areas
- **Top (7): A8 Auth, A11 Services** — thin, correct wrappers over good crates; security
  primitives verified correct (argon2 OWASP params; fixed-alg JWT; no hand-rolled crypto).
- **Bottom (5): A2 HTTP, A6 Storage, A7 Data, A9 Config, A12 Testing** — real correctness
  bugs + heavy internal duplication + thin test coverage of the affected paths.

## Feeds Iteration 2
- Library hypotheses to spike (build real throwaway crates, no assumptions): tower_http
  request-id, axum-client-ip, FuturesUnordered (storage fan-out), moka::future (cache),
  num-format (number filter), apalis (bgworker), Cookie::value + axum-valid (auth), url
  crate + ColType→sea_orm builder (data), cargo_metadata + crates.io JSON API (tooling),
  ptree/termtree + bon (codegen/testing).
- Findings still to adversarially refute (guard against false-positives like the bool arm
  near-miss): redis 3s-boot-sleep, reference_id normalized-vs-raw, Error→HTTP 500 collapse.
