# Loco 1.0 — Master Feature Inventory & Doc-Coverage Matrix

Synthesized from the 8 verified subsystem inventories (01–08) in this folder.
Every rating is code-checked. Constraint honored: **only verified facts**; each row
traces to a `file:line` in its source inventory.

Legend: **ACCURATE** (matches code) · **THIN** (correct but undersells API) ·
**STALE** (contradicts current code — actively misleading) · **MISSING** (no page).

## A. Coverage matrix (feature → current page → rating)

| Feature / subsystem | Current doc page | Rating | Headline issue |
|---|---|---|---|
| App & `Hooks` (full surface) | the-app/your-project, pluggability | THIN | `init_logger`/`load_config`/`after_context`/`before_run` undocumented; some snippets use stale `environment:&str` |
| `AppContext` (8 fields) | scattered | STALE | `shared_store`, `cache` fields not listed; `db` feature-gating unnoted |
| **SharedStore / DI** (store + extractor) | — | **MISSING** | fully implemented + tested, zero docs |
| Controllers & routing | the-app/controller (1418L) | ACCURATE/THIN | `merge`/`merge_all`/`Routes::layer` newer; verify shown |
| Error→HTTP status map + `ErrorDetail` body | the-app/controller | THIN | exact status map + JSON body shape `{error,description,errors}` unspecified |
| Format/response helpers | the-app/controller | THIN | `yaml`, `empty_json`, `redirect_with_header_key`, cookies, `render().response()` missing |
| **Middleware `--config` sample** | the-app/controller:348 | **STALE** | shows `expose_header` (singular) vs real `expose_headers` |
| **Monitoring `/_ping /_health /_readiness`** | — | **MISSING** | 503-on-dependency-failure semantics undocumented |
| Views / Tera | the-app/views | THIN | `embedded_assets` engine swap, `build_with_post_process` missing |
| Prelude reference | axum-users | THIN | no exhaustive prelude/AppContext reference |
| Models / ActiveModel | the-app/models (1092L) | THIN | `ModelError`/`ModelResult`/`Authenticable` never documented |
| **Field-type table (`int`→i64)** | the-app/models:170 | **STALE** | shows `int` as 32-bit; code maps `int`→**big_integer/i64** (1.0 change) |
| **Query DSL + Pagination** | — | **MISSING** | `ConditionBuilder` (~18 ops), `paginate`/`PaginationQuery` rustdoc-only |
| Migration schema DSL (`ColType`) | the-app/models | THIN | ~140 variants; `add_reference`/`add_enum_values` undocumented |
| DB config keys | the-app/models | STALE | omits `acquire_timeout`, `run_on_start`; pool defaults unsurfaced |
| Background workers | processing/workers (372L) | STALE | **priority queues entirely missing**; `perform_later` shown `Result<()>` but returns `Result<String>` (job id) |
| Worker backend config knobs | processing/workers | STALE | docstrings claim "Redis not supported" for cancel/clear/requeue — Redis IS implemented |
| Scheduler | processing/scheduler | ACCURATE | — |
| Tasks | processing/task | ACCURATE/THIN | "lists executed" should read "registered" |
| **Mailer TLS (implicit/465)** | processing/mailers | **STALE** | omits `tls` mode; `secure:true` no longer covers all TLS — actively misleads |
| Mailer headers/cc/bcc/priority | processing/mailers | THIN | `EmailHeaders`, cc/bcc, `hello_name`, job priority missing |
| Storage streaming + drivers | infrastructure/storage | STALE | driver ctor signatures wrong (`aws::new("users")` vs real `new(bucket,region)`); trait misnamed `StorageDriver` vs real `StoreDriver`; streaming API absent |
| Cache API | infrastructure/cache (116L) | THIN | `get_or_insert_with_expiry`, `ping`, `clear` (Redis=FLUSHDB caveat) missing |
| **Authentication** | extras/authentication (257L) | THIN/STALE | feature named `auth` not real `auth_jwt`; HS512 default + base64-secret req missing; `JWTWithUser`, `Authenticable`, JWT locations, `hash` module all missing |
| **Configuration reference** | your-project (defers to rustdoc) | **MISSING** | no enumerated config reference — the single biggest gap; Part B of inv-05 is the replacement |
| Env/secrets model | your-project | THIN | `LOCO_ENV`→`RAILS_ENV`→`NODE_ENV` precedence, `.local.yaml` tier, `get_env` = Tera builtin undocumented |
| **Testing** | models.md#testing only | **MISSING/STALE** | no page; examples use non-compiling `boot_test::<App,Migrator>()` (real: single-generic `boot_test<H>()`) |
| Initializers | pluggability | ACCURATE/THIN | newer `check()` doctor hook unnoted |
| Diagnostics (`doctor`) | — | MISSING | 6 checks + flags undocumented |
| `data`/`tera`/`cargo_config` | — | MISSING | loaders + entity-metadata knobs |
| **Generators reference** | scattered | **MISSING** | no single page; ~50 field types; `override`/`deployment`/`data` near-invisible |
| **CLI reference** | your-project (--help exec) | THIN | per-subcommand flags (`db seed --dump`, `jobs purge`, `start --all`) missing |
| **Feature-flag matrix** | scattered | **MISSING** | no central matrix; `auth_jwt`/`bg_*`/`testing` default-status undocumented |
| App creation (`loco new`) | getting-started/starters | STALE | invents `--template`/`--verbose`; omits `--os`, `--allow-in-git-repo`, `Advanced` template, `none` values |

## B. Cross-cutting 1.0 truths every page must reflect
1. **`int` field-type is now i64 / BIGINT** (was i32). Highest-frequency stale fact — audit every `field:type` example framework-wide.
2. **64-bit (i64) primary keys + BigInteger FKs** (Sea-ORM 2.0 + sqlx 0.9).
3. **Priority queues** across all 3 backends; `perform_later` returns a job-id `String`; Redis now fully supports cancel/clear/requeue.
4. **Mailer TLS modes** (`starttls`/`implicit`/`none`, port 465) override the legacy `secure` bool.
5. **`auth_jwt`** is the real feature name; default JWT alg **HS512**; secret must be **base64**; multi-location JWT (Bearer/Query/Cookie).
6. **Error enum narrowed** (`#[non_exhaustive]`; removed `EnvVar`/`Hash`/`SemVer`/`TaskJoinError`).
7. **edition 2024**: any `std::env::set_var` snippet needs `unsafe {}`. Generated apps still edition 2021 (bump candidate).
8. **`embedded_assets`** swaps view engine + static middleware to embedded variants.

## C. Gap priority (authoring order for 1.0)
**Tier 1 — new reference pages (highest value, currently MISSING):**
1. Configuration reference (from inv-05 Part B — exhaustive, verified).
2. Generators & field-type reference (from inv-07).
3. CLI reference + feature-flag matrix (from inv-08).
4. Testing guide (from inv-06) — as its own page.
5. Authentication & security rewrite (from inv-05 Part A).

**Tier 2 — STALE fixes on existing strong pages (correctness):**
6. workers.md (priority queues, job-id return, Redis support).
7. mailers.md (TLS modes, headers).
8. storage.md (driver signatures, `StoreDriver`, streaming).
9. models.md field-type table (`int`→i64) + Query DSL/Pagination + `ModelError`.
10. controller.md middleware `--config` sample (`expose_headers`).

**Tier 3 — THIN enrichments:**
11. AppContext/prelude reference; SharedStore/DI; monitoring endpoints; Hooks full surface; cache API; logging/observability; initializers `check()`.

## D. Engineering cleanups surfaced (not docs — flag to maintainer)
- `src/initializers/extra_db.rs:19,25,30` — leftover `println!("1"/"2"/"3")` debug lines.
- `RequestConfig.default_scheme` never forwarded to axum-test's `TestServerConfig` (inv-06).
- `src/controller/describe.rs` regex depends on axum 0.8 `Debug` format — fragile; `cargo loco routes` method display can silently break on axum upgrade.
- `getting-started/starters.md` hand-written `--help` block drifted — convert to an exec snippet.
