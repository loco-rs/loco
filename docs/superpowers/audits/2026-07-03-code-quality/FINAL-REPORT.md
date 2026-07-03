# Loco Code-Quality Audit — Final Report (Iteration 3, reconciled)

**Method.** 3 iterations, governed fan-out. Iter-1: 12 independent area reviews scored
against 7 KPIs, every claim cited `file:line`. Iter-2: adversarial refutation of the top
findings + 12 library hypotheses each validated by a REAL compiled throwaway crate (no
library judged by assumption). Iter-3 (this doc): reconcile, apply the adjustments the
evidence justifies, lock scores. Sonnet did the grunt review/spike work; Opus governed —
personally re-verified the highest-stakes correctness claims against source and caught the
one near-false-positive (the Postgres bool arm), which then held under tracing.

**Headline.** Loco is genuinely well-architected: strong crate selection and clean domain
boundaries. Its weak axis is exactly the "patch-on-patch" the brief predicted — brittleness
and correctness/test-coverage — concentrated in duplicated code that drifted out of sync
and in untested "shell" wiring. **No iteration-1 finding was overturned as a false-positive**
under adversarial refutation; several were reinforced with fresh reproduction.

## Locked scorecard (iter-1 scores, validated by iter-2 — unchanged except A7 K6 3→ noted)

| Area | K1 Vision | K2 Econ | K3 LOC | K4 Brittle | K5 Maint | K6 Correct | K7 NoReinvent | **Overall** |
|------|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| A1 Boot & Lifecycle    | 7 | 7 | 6 | 4 | 7 | 4 | 7 | **6** |
| A2 HTTP & Routing      | 5 | 6 | 6 | 3 | 5 | 4 | 4 | **5** |
| A3 Middleware          | 6 | 6 | 5 | 6 | 7 | 4 | 5 | **6** |
| A4 Views               | 5 | 6 | 6 | 4 | 5 | 5 | 6 | **6** |
| A5 Background Workers    | 5 | 6 | 3 | 4 | 5 | 6 | 5 | **6** |
| A6 Storage             | 5 | 4 | 3 | 3 | 5 | 4 | 8 | **5** |
| A7 Data Layer          | 5 | 4 | 5 | 3 | 6 | 3 | 5 | **5** |
| A8 Auth & Validation   | 6 | 5 | 6 | 5 | 7 | 7 | 7 | **7** |
| A9 Config & Observ.    | 5 | 6 | 7 | 3 | 6 | 3 | 5 | **5** |
| A10 CLI & Codegen      | 6 | 6 | 4 | 5 | 6 | 6 | 7 | **6** |
| A11 Services           | 6 | 8 | 6 | 6 | 7 | 7 | 9 | **7** |
| A12 Testing            | 6 | 7 | 7 | 4 | 6 | 4 | 6 | **5** |
| **Mean**               | **5.6** | **5.9** | **5.2** | **4.2** | **6.0** | **4.75** | **6.2** | **5.6** |

(Only change from iter-1: A7 Correctness 4→3, justified by 5 confirmed correctness defects
clustered in schema/connect; A7 Overall held at 5 — the query DSL and seed code are sound.)

**KPI profile:** strongest = No-reinvention (6.2) & Maintainability/DDD (6.0); weakest =
Non-brittle (4.2) & Correctness (4.75). This is the precise fingerprint of sound design +
evolutionary cruft.

## Confirmed defect register (all verified against source; ✓ = Opus re-verified personally)

| # | Sev | Area | Defect | Cite |
|---|-----|------|--------|------|
| 1 | High | A6 | `mirror` rename/copy check `should_fail` INSIDE the secondary loop (early-return) while upload/delete check AFTER — under MirrorAll, a first-secondary failure leaves later mirrors silently stale ✓ | `storage/strategies/mirror.rs:174-176,205-207` |
| 2 | High | A7 | `dump_tables` type-probe chain has no `bool` arm; PG `BOOLEAN` decodes as nothing → column silently dropped from schema dump; masked by SQLite-only test ✓ | `db/schema.rs:196-238` |
| 3 | High | A1 | `Hooks::on_shutdown` never fires in `WorkerOnly`/`WorkerAndScheduler` — boot bypasses `H::serve`, its only caller ✓ | `boot.rs:140-146`, `app.rs:346` |
| 4 | High | A9 | `Config::from_folder` first-existing-file-wins, NO merge — `.local.yaml` must restate entire config or fail ✓ | `config/mod.rs:153-174` |
| 5 | High | A2 | `describe.rs` `.captures()` first-match-only drops all-but-first HTTP verb from `cargo loco routes` (introspection only, not dispatch); reproduced vs axum 0.8.9 | `controller/describe.rs:23` |
| 6 | High | A9/A2 | `IntoResponse for Error`: only 7 of 35 variants get distinct status; 28 collapse to 500 incl. `Model(EntityNotFound)` → wrongful 500 in Loco's OWN demo `current()` | `controller/mod.rs:204-249` |
| 7 | Med-High | A7 | admin-DB URL via whole-string `.replace(db_name,"/postgres")` corrupts when a role/host contains the db-name substring ✓ | `db/connect.rs:173` |
| 8 | Med-High | A7 | `reference_id()` fed normalized name in create_table but raw names in add/remove_reference → irregular plurals (person/people) yield mismatched FK names (proven vs cruet 1.0) | `schema.rs:648,780,865` |
| 9 | Med | A1 | fallback middleware default status is 200 despite docs/comments promising 404 ✓ | `middleware/fallback.rs:39-40` |
| 10 | Med | A4 | `ViewEngine` extractor declares `Rejection=Infallible` but `.expect()`-panics if the (opt-in) Tera layer is absent; reachable in supported configs, untested | `controller/views/mod.rs:74,82` |
| 11 | Med | A5 | fossilized `sleep(3s)` on EVERY Redis boot (git-blamed to a test-isolation commit that leaked to prod); no pg/sqlt equivalent | `bgworker/redis.rs:971` |
| 12 | Med | A5 | no automatic visibility-timeout reaper in any backend — a crashed worker strands jobs in `Processing` until manual `cargo loco jobs requeue` | `bgworker/*`, `cli.rs:1291` |
| 13 | Med | A12 | Postgres test-db cleanup is fire-and-forget (`spawn_blocking` + new Runtime, never awaited) → can leak test DBs; its test masks the race with `sleep(1s)` | `testing/db.rs:109-127,246-249` |
| 14 | Low-Med | A12 | password redaction regex `password: (.*{60}),` is inert (≡ `.*`), zero tests (verified by compiling regex) | `testing/redaction.rs:15` |
| 15 | Low | A8 | `JWT::algorithm()` builder is a dead-end footgun: asymmetric algs can't work (encode/decode hardcoded to base64 secret) | `auth/jwt.rs:49-54,83,108` |

## Dead / fossil code shipped in-tree (confirmed)
- `middleware/_archive/content_etag.rs` (103 LOC, no `mod` decl, `// Corrected import`) ✓
- `backtrace.rs:18-35` — 17 commented-out regex lines
- `views/mod.rs:83-92` — abandoned locale block with `// BUG: does not mutate ... because of clone`
- `schema.rs:792,805,817` — `// xxx fix` / `// XXX fix, totbl_id` migration-DSL patch comments

## Duplication-that-drifted (the core patch-on-patch theme; each is an internal-DRY target)
- Storage `backup.rs` vs `mirror.rs` — ~8 copy-pasted fan-out methods; the copy diverged into defect #1.
- Bgworker `redis.rs:36-236` re-derives the Job/Registry/poll-loop that `sql.rs` generalizes; `pg.rs`/`sqlt.rs` `enqueue`/`to_job`/`fail_job` 85-100% identical.
- HTTP `format.rs` — 6 response concepts implemented twice (free fns vs `RenderBuilder`), already diverged; 2 have no builder equivalent.
- Boot twin `create_app` (`boot.rs:403-437`); CLI twin `main` (`cli.rs:712-869` vs `:872-1015`).
- Auth two JWT validate paths (`auth.rs:67-99` vs `:126-146`); 6 near-identical validate extractors.
- Config `env_vars.rs` constants re-duplicated in `environment.rs:22-24`.

## Library hypotheses — verdicts from REAL compiled spikes (crates under scratchpad/spikes/)

| Hyp | Verdict | Evidence |
|-----|---------|----------|
| H6 `Cookie::value()` (auth) | **PROVEN-FIT** | byte-identical incl. adversarial cases; already-imported dep; −5 LOC |
| H9 `url` crate (connect) | **PROVEN-FIT** | fixes defect #7; 0 new deps (transitive via sqlx); net ~0 |
| H11 `ureq`+`semver` (doctor) | **PROVEN-FIT** | fixes pre-release truncation (affects Loco's own 2.0.0-rc check); caveat: dev→runtime dep |
| H5 `moka::future` (cache) | **PROVEN-FIT** | removes sync-behind-async smell; just enable moka `future` feature; net ~0 |
| H4 `join_all` (storage) | **PARTIAL→win** | unifies 6/8 fan-out loops −40-60 LOC AND fixes defect #1 (the "lost" early-exit IS the bug) |
| H1 tower-http request-id | DOESN'T-FIT | must reimplement Rails sanitize verbatim + a no-header-drop footgun; net flat |
| H2 axum-client-ip | DOESN'T-FIT | 1.0 removed trusted-proxy multi-hop skip; diverges on Loco's own XFF vectors |
| H3 apalis (bgworker) | DOESN'T-FIT | can't express runtime string-keyed registry; no priority/tags/admin ops; net 0..+100 |
| H7 num-format | DOESN'T-FIT | drops sign on `-0.123`; caps at i128 vs unbounded string grouping |
| H8 axum-valid | DOESN'T-FIT | can't produce Loco's two error tiers (`{"errors":…}` vs `{"error":…}`) |
| H10 cargo_metadata | DOESN'T-FIT | trades sub-ms file read for ~11ms subprocess; same LOC |
| H12 termtree | DOESN'T-FIT | can't express collapsing + fixed-width 3-col layout; needs more code |

**Net: 5 wins (all already-in-tree deps or bug-fixers), 7 rejections (all validating Loco's
lean choices).** The rejections are a *positive* audit result: Loco's hand-rolled queue,
remote-IP, number filter, and validation tiers are genuine product value no crate replaces
AS SIMPLY — its KPI7 (6.2, the top score) is earned, not lucky.

## Prioritized remediation roadmap (what to actually do)

**P0 — correctness bugs (ship fixes; several are one-liners):**
1. `fallback` default 404 not 200 (#9) — 1 line.
2. Add a `bool` arm to `dump_tables` (#2) — a few lines; add a PG-backed test.
3. `on_shutdown` in worker-only modes (#3) — route worker-only shutdown through the hook.
4. Adopt `url` crate in `connect::create` (#7) — PROVEN-FIT, 0 new deps.
5. Expand `Error→HTTP` mapping (#6) — give `EntityNotFound`/form-rejection/etc. real statuses.
6. `reference_id` normalization consistency (#8) — normalize at all 3 call sites.

**P1 — brittleness & robustness:**
7. Storage: unify backup/mirror fan-out via `join_all` (H4) — kills defect #1 AND −50 LOC.
8. Bgworker: add a visibility-timeout reaper (#12); remove the fossil `sleep(3s)` (#11).
9. Config: implement `.local.yaml` deep-merge or rename to signal replace-semantics (#4).
10. `doctor`: `ureq`+`semver` for crates.io version check (H11) — fixes the rc-truncation.

**P2 — internal DRY (no external deps; addresses the K1/K3 duplication scores):**
11. Bgworker: make `redis.rs` implement the `Driver` trait; factor pg/sqlt shared CRUD.
12. Collapse the 6 validate extractors + 2 JWT paths + twin `main`/`create_app` + format.rs's doubled API.
13. Delete fossil code (`_archive/`, backtrace comments, locale block, xxx-fix comments).

**P3 — small proven wins:** `Cookie::value()` (H6), `moka::future` (H5).

**Test-coverage debt (raises K6 across the board):** the confirmed bugs live in untested
files — `boot.rs`, `schema.rs`, `cli.rs`, 10/16 middleware, `engine_embedded.rs`,
`redaction.rs`, `config/mod.rs`, `errors.rs` all lack or under-test the affected paths.

## What is genuinely excellent (do not touch)
- Auth security primitives: argon2 at OWASP params, fixed-alg JWT (no alg-confusion), no
  hand-rolled crypto (A8, verified against crate source).
- Services as thin correct wrappers: scheduler over tokio_cron_scheduler+english_to_cron,
  cache over moka/bb8, mailer over lettre (A11 — top KPI7 = 9).
- `SharedStore` DashMap container — the best-tested code in boot; no crate improves it.
- The `Driver` trait abstraction for pg+sqlt queues (the RIGHT model; redis just sits outside it).
- OpenDAL-thin storage driver layer; Sea-ORM-thin data layer where it isn't reinventing.
