# Inventory 06 — Testing, Initializers, Diagnostics, Data & Misc

Area owner: Testing support, initializers, diagnostics, data & misc.
Verified against source at branch `release/0.17.0`. All `file:line` refer to the actual code read.

---

## A. Testing support (`src/testing/`)

Feature-gated behind the `testing` Cargo feature = `["dep:axum-test", "dep:scraper", "dep:tree-fs"]` (`Cargo.toml:43`). DB-touching parts additionally gated on `with-db`. axum-test pinned to `17.0.1` (`Cargo.toml:133`).

Module tree (`src/testing/mod.rs:1-6`):
- `db` (only `#[cfg(feature = "with-db")]`), `prelude`, `redaction`, `request`, `selector`.

Prelude (`src/testing/prelude.rs:1-3`): re-exports `db::*` (with-db only), `redaction::*`, `request::*`, `selector::*`. Users import `use loco_rs::testing::prelude::*;`.

### A.1 Boot helpers — `src/testing/request.rs`
| Fn | Signature | file:line | Notes |
|---|---|---|---|
| `boot_test` | `async fn boot_test<H: Hooks>() -> Result<BootResult>` | `request.rs:185` | Boots `StartMode::ServerOnly` in `Environment::Test`. Single generic `H` (NOT `<App, Migrator>`). |
| `boot_test_with_create_db` | `async fn ...<H: Hooks>() -> Result<BootResultWrapper>` | `request.rs:209` | `with-db` only. Creates a fresh DB/schema, rewrites `config.database.uri`, cleans up on drop. |
| `boot_test_unique_port` | `async fn ...<H: Hooks>(port: Option<i32>) -> Result<BootResult>` | `request.rs:248` | Overrides `config.server`; default port `TEST_PORT_SERVER` (5555). |

`BootResultWrapper` (`request.rs:20-50`, with-db only): wraps `BootResult` + `Box<dyn TestSupport>`; `Deref` to `BootResult` (`:37`); `Drop` calls `test_db.cleanup_db()` (`:46-49`) — automatic DB teardown.

### A.2 Request helpers — `src/testing/request.rs`
| Fn | file:line | Notes |
|---|---|---|
| `request<H: Hooks>(callback)` | `:302` | Default `RequestConfig`, no DB creation. Callback is `AsyncFnOnce(TestServer, AppContext)`. |
| `request_with_config<H>(config, callback)` | `:347` | Custom `RequestConfig`. |
| `request_with_create_db<H>(callback)` | `:327` | with-db only; fresh DB. |
| `request_config_with_create_db<H>(config, callback)` | `:373` | with-db only; custom config + fresh DB. |
| `get_available_port() -> i32` (async) | `:152` | Binds `localhost:0`, returns ephemeral port. Panics on bind failure. |
| `get_base_url_port(port) -> String` | `:143` | `http://localhost:{port}/`. |

Constants: `TEST_PORT_SERVER: i32 = 5555` (`:136`), `TEST_BINDING_SERVER: &str = "localhost"` (`:139`).

`RequestConfig` (`:53-60`) + `RequestConfigBuilder` (`:69-122`): fields `save_cookies: bool` (default false), `default_content_type: Option<String>` (default `"application/json"`), `default_scheme: String` (default `"http"`). Builder methods `.save_cookies()`, `.default_content_type()`, `.default_scheme()`, `.build()`. `From<RequestConfig> for TestServerConfig` (`:125-133`) — note: only maps `default_content_type` + `save_cookies`; `default_scheme` is NOT forwarded to axum-test.

### A.3 DB test support — `src/testing/db.rs` (with-db only)
| API | file:line | Notes |
|---|---|---|
| `seed<H: Hooks>(ctx) -> Result<()>` | `:36` | Hardcodes fixtures path `src/fixtures`, calls `H::seed(ctx, path)`. |
| `init_test_db_creation(conn_str) -> Result<Box<dyn TestSupport>>` | `:45` | Dispatch by URI prefix: `postgres://`→`PostgresTest`, `sqlite://`→`SqliteTest`, else `Any`. |
| `trait TestSupport: Send + Sync` | `:55-62` | Methods: `init_db()` (async via pinned future), `get_connection_str()`, `cleanup_db()`. |
| `PostgresTest` | `:64-129` | Creates a uniquely-named DATABASE `_loco_test_{rand10}_{unix_ts}` via `hash::random_string(10)` + timestamp (`:78-80`). Connects as root DB `postgres`, `CREATE DATABASE`; cleanup drops it on a spawned blocking runtime. |
| `SqliteTest` | `:131-177` | Backs DB with a `tree_fs` temp file `test.sqlite`; `init_db` is a no-op; cleanup removes the temp dir. |
| `Any` | `:179-200` | Passthrough for unknown schemes; all ops no-op. |

### A.4 HTML/selector assertions — `src/testing/selector.rs` (scraper-based)
All panic-on-failure assertion helpers:
- `assert_css_exists(html, selector)` `:24`
- `assert_css_not_exists(html, selector)` `:54`
- `assert_css_eq(html, selector, expected_text)` `:84`
- `assert_link(html, selector, expected_href)` `:126` (delegates to `assert_attribute_eq` on `href`)
- `assert_attribute_exists(html, selector, attribute)` `:155`
- `assert_attribute_eq(html, selector, attribute, expected_value)` `:198`
- `assert_count(html, selector, expected_count)` `:245`
- `assert_css_eq_list(html, selector, &[&str])` `:283`
- `select(html, selector) -> Vec<String>` `:322` (returns outer HTML of matches)

### A.5 Snapshot redaction filters — `src/testing/redaction.rs`
For use with `insta`'s `with_settings!{ filters => ... }`. Loco owns the regex→placeholder tables; `insta` itself is a dev-dependency of the app, not re-exported.
- `cleanup_user_model() -> Vec<(&str,&str)>` `:90` — combines user-model + date + model filters (PID/UUID→`PID`, password→`PASSWORD`, JWT→`TOKEN`, timestamps→`DATE`, `id: N`→`id: ID`).
- `cleanup_email() -> Vec<(&str,&str)>` `:99` — mail identifiers + dates.
- Backing lazily-init tables (public): `get_cleanup_user_model()` `:8`, `get_cleanup_date()` `:21`, `get_cleanup_model()` `:34`, `get_cleanup_mail()` `:38`.

### A.6 Other test-related flags
- `integration_test = []` feature (`Cargo.toml:62`) — empty marker feature, gates integration-test-only code paths elsewhere in the crate (no test API of its own in this area).

---

## B. Initializers (`src/initializers/` + trait in `src/app.rs`)

### B.1 The `Initializer` trait — `src/app.rs:453-477`
```rust
pub trait Initializer: Sync + Send {
    fn name(&self) -> String;                                                   // :455 (required)
    async fn before_run(&self, _app_context: &AppContext) -> Result<()>         // :460 default Ok(())
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext)
        -> Result<AxumRouter>                                                   // :467 default Ok(router)
    async fn check(&self, _app_context: &AppContext)
        -> Result<Option<crate::doctor::Check>>                                 // :474 default Ok(None)
}
```
- `check` is the doctor integration hook (see §C). Registered via `Hooks::initializers(ctx) -> Result<Vec<Box<dyn Initializer>>>` (`src/app.rs:395`, default returns empty vec).

### B.2 Built-in initializers (`#[cfg(feature = "with-db")]`)
Only two ship in loco-rs core; `mod.rs:1-6` gates both on `with-db`.
| Struct | name() | Hook | Config key | file:line |
|---|---|---|---|---|
| `ExtraDbInitializer` | `"extra_db"` | `after_routes` — reads `config.initializers["extra_db"]`, `db::connect`, layers `Extension(db)` | `initializers.extra_db` | `extra_db.rs:10-36` |
| `MultiDbInitializer` | `"multi_db"` | `after_routes` — reads `config.initializers["multi_db"]`, `db::MultiDb::new`, layers `Extension(multi_db)` | `initializers.multi_db` | `multi_db.rs:10-32` |

Both require the `initializers:` config section; error with `Error::Message(...)` if missing.

**Code smell / 1.0 cleanup:** `ExtraDbInitializer::after_routes` contains leftover debug `println!("1")`, `println!("2")`, `println!("3")` at `extra_db.rs:19,25,30`. Neither built-in implements `check()`.

---

## C. Diagnostics — `src/doctor.rs`

Runs behind `cargo loco doctor` (`src/cli.rs:814-834`; also `:970`). CLI flags: `--config` (prints resolved config + environment, skips checks) and `--production` (skips dev-only checks). Non-zero exit if any check is invalid (`cli.rs:822-831`).

### C.1 Public types
- `enum Resource` `:100-108`: `SeaOrmCLI, Database, Queue, Deps, PublishedLocoVersion, Initializer(String)`.
- `enum CheckStatus` `:111-116`: `Ok, NotOk, NotConfigure`.
- `struct Check { status, message, description: Option<String> }` `:119-127`; `Check::valid()` `:131` (true unless `NotOk`), `Check::to_result()` `:139`, `Display` renders ✅/❌/⚠️ (`:152-171`).

### C.2 Checks
- `run_all<H: Hooks>(app_context, production) -> Result<BTreeMap<Resource, Check>>` `:176` — orchestrates: DB (with-db), Queue (if `WorkerMode::BackgroundQueue`), initializer `check()`s (message prefixed `Initializer {name}: `, `:199`), and — only when `!production` — Deps, SeaOrmCLI, PublishedLocoVersion.
- `check_db(&config.database) -> Check` `:259` (with-db) — connect → ping → `verify_access`.
- `check_queue(&Config) -> Check` `:291` — via `bgworker::create_queue_provider` + ping; `NotConfigure` if absent.
- `check_seaorm_cli() -> Result<Check>` `:318` — runs `sea-orm-cli --version`; min version `MIN_SEAORMCLI_VER = "2.0.0-rc"` (`:39`). Fix hint: `cargo install sea-orm-cli`.
- `check_deps() -> Result<Check>` `:222` — via `depcheck::check_crate_versions("Cargo.lock", ...)`.
- `check_published_loco_version() -> Result<Check>` `:369` — compares `CARGO_PKG_VERSION` against crates.io.
- `check_cratesio_version(crate_name, current) -> Result<Option<String>>` `:65` — parses `cargo search` output.

Min blessed versions (`get_min_dep_versions()` `:47-58`): `tokio 1.33.0`, `sea-orm 2.0.0-rc`, `validator 0.20.0`, `axum 0.8.1`. **1.0 note:** these still pin `sea-orm`/sea-orm-cli to `2.0.0-rc`, not `2.0.0` stable — consistent with the release being gated on Sea-ORM 2.0.0 stable.

### C.3 depcheck — `src/depcheck.rs`
Own error type + Result: `enum VersionCheckError { LockfileError(String), CrateError{crate_name,msg} }` (`:30-37`), `pub type Result<T>` (`:39`). `enum VersionStatus { NotFound, Invalid{version,min_version}, Ok(String) }` (`:14-22`), `struct CrateStatus { crate_name, status }` (`:24-28`). `check_crate_versions(lock_path, HashMap<&str,&str>) -> Result<Vec<CrateStatus>>` (`:50`) — reads `Cargo.lock` via the `cargo-lock` crate. `VersionCheckError` flows into the app `Error` via `Error::VersionCheck` (`errors.rs:141`).

---

## D. Error surface — `src/errors.rs` (1.0-critical)

- `pub type Result<T, E = Error> = std::result::Result<T, E>` — defined in `src/lib.rs:52`; `pub use self::errors::Error` (`lib.rs:5`).
- `enum Error` is `#[non_exhaustive]` (`errors.rs:31`) — downstream `match` must have a wildcard arm (breaking as of commit `e476e205`).

**Current variants (`errors.rs:32-151`), verified:**
`WithBacktrace{inner,backtrace}` `:34`, `Message(String)` `:40`, `QueueProviderMissing` `:46`, `TaskNotFound(String)` `:49`, `Scheduler(#[from])` `:52`, `Axum(#[from] http::Error)` `:55`, `Tera(#[from])` `:58`, `JSON(serde_json::Error)` `:61` (hand-rolled `From` at `:24-28` to capture backtrace), `JsonRejection(#[from])` `:64`, `YAMLFile(source, String)` `:67`, `YAML(#[from])` `:70`, `EmailSender(#[from] lettre)` `:73`, `Smtp(#[from])` `:76`, `Worker(String)` `:79`, `IO(#[from])` `:82`, `DB(#[from] sea_orm::DbErr)` `:86` (with-db), `ParseAddress(#[from])` `:89`, `Unauthorized(String)` `:93`, `NotFound` `:97`, `BadRequest(String)` `:100`, `CustomError(StatusCode, ErrorDetail)` `:103`, `InternalServerError` `:106`, `InvalidHeaderValue`/`InvalidHeaderName`/`InvalidMethod` `:109/:112/:115`, `Model(#[from] ModelError)` `:120` (with-db), `Redis(#[from])` `:124` (bg_redis), `Sqlx(#[from])` `:128` (bg_pg/bg_sqlt), `Storage(#[from])` `:131`, `Cache(#[from])` `:134`, `Generators(#[from] loco_gen::Error)` `:138` (debug_assertions), `VersionCheck(#[from])` `:141`, `Any(#[from] Box<dyn Error+Send+Sync>)` `:144`, `Validation(#[from] ModelValidationErrors)` `:147`, `AxumFormRejection(#[from])` `:150`.

Constructors: `Error::wrap(err)` `:154`, `Error::msg(err)` `:158`, `Error::string(&str)` `:162`, `Error::bt(self)` `:166` (captures backtrace only when `RUST_BACKTRACE` set).

**REMOVED in commit `4a4a84ee` ("narrow the Error enum — drop 4 low-value/leaky variants") — confirmed absent from current source:**
- `EnvVar(#[from] std::env::VarError)`
- `Hash(String)`
- `TaskJoinError(#[from] tokio::task::JoinError)`
- `SemVer(#[from] semver::Error)`

Migration implication for 1.0 docs: code that matched or constructed any of these four no longer compiles; combined with `#[non_exhaustive]`, downstream `match Error` must add a `_ =>` arm.

---

## E. Data & misc

- **`src/data.rs`** — JSON data loading. `load_json_file_sync<T: DeserializeOwned>(path) -> Result<T>` (`:17`), `load_json_file<T>(path)` async (`:30`). Both resolve relative to a data folder: `DEFAULT_DATA_FOLDER = "data"` (`:7`), overridable via env var `env_vars::LOCO_DATA_FOLDER_ENV` (`data_folder()` `:8-10`). Exposed as `pub mod data` (`lib.rs:13`).
- **`src/tera.rs`** — one-liner templating helper. `render_string(tera_template: &str, locals: &serde_json::Value) -> Result<String>` (`:5`) via `Tera::one_off(..., autoescape=false)`. Very small surface.
- **`src/cargo_config.rs`** — `Cargo.toml` reader for generator/entity metadata. `struct CargoConfig` (`:19`); `from_current_dir()` `:29`, `from_path(path)` `:38`, `get_db_entities() -> Option<&Table>` reads `[package.metadata.db.entity]` (`:53`). Uses app `Error`/`Result` (`:10-11`). Note module doc points to `depcheck` for Cargo.lock parsing.

---

## F. Doc coverage assessment ("only VERIFIED docs")

| Feature | Current doc location | Rating | Concrete discrepancy |
|---|---|---|---|
| Testing API (boot/request/DB/selector/redaction) | `the-app/models.md#testing` only — NO dedicated page | **THIN + STALE** | (1) All examples use `boot_test::<App, Migrator>()` (`models.md:847,972,1027,1041`) but real signatures take ONE generic: `boot_test<H: Hooks>()` (`request.rs:185`). Two-type-param form does not compile. (2) `seed::<App>` shown as `seed::<App>(&boot.app_context)` — matches `db.rs:36` OK. (3) No page documents `request*`, `RequestConfigBuilder`, `boot_test_unique_port`, `get_available_port`, or the full selector-assertion family. (4) axum-test version (17.x) and `default_scheme`-not-forwarded gotcha undocumented. |
| Selector/HTML assertions | none (only rustdoc in `selector.rs`) | **MISSING** | 9 public assert fns + `select()` have zero prose docs. |
| Snapshot redaction | `models.md` (Snapshot testing section ~`:1052`) | **THIN** | `cleanup_user_model`/`cleanup_email` mentioned; the `get_cleanup_*` tables and exact placeholders not documented. |
| Initializer trait | `extras/pluggability.md:114-310` | **ACCURATE** | Trait signature, `check()` doctor integration, and `initializers()` registration all match `app.rs:453-477`. Examples use template-level initializers (AxumSession/ViewEngine), which live in the generated app, not loco-rs core. |
| Built-in `extra_db`/`multi_db` initializers | not documented as built-ins | **MISSING** | The two `with-db` built-ins and their `initializers.{extra_db,multi_db}` config keys are undocumented. |
| `cargo loco doctor` | pluggability.md (initializer-check angle only) | **THIN** | No page enumerates the `Resource`/`CheckStatus`/`Check` model, `--config`/`--production` flags, or the six built-in checks + blessed min-versions. |
| Error enum (narrowed) | scattered | **STALE risk** | Any doc/example referencing `Error::EnvVar/Hash/TaskJoinError/SemVer` is now wrong; `#[non_exhaustive]` requirement for downstream matches undocumented. Must document the current 30+ variant surface + 4 removals for the 0.16→0.17 migration guide. |
| `data.rs` / `tera::render_string` / `cargo_config` | none | **MISSING** | `load_json_file[_sync]`, `LOCO_DATA_FOLDER` env var, `render_string`, and `CargoConfig` are undocumented. |

---

## 10-line summary

1. Testing API is fully in `src/testing/` (feature `testing`): boot helpers (`boot_test`, `boot_test_with_create_db`, `boot_test_unique_port`), request helpers (`request`, `request_with_config`, `*_with_create_db`), `RequestConfigBuilder`, `get_available_port`.
2. DB test support (`db.rs`, with-db): `seed`, `init_test_db_creation`, `trait TestSupport`, `PostgresTest`/`SqliteTest`/`Any`; `BootResultWrapper` auto-cleans DB on drop.
3. `selector.rs` = 9 panic-assert HTML helpers + `select()`; `redaction.rs` = `cleanup_user_model`/`cleanup_email` for insta snapshots.
4. Initializer trait (`app.rs:453`): `name`+`before_run`+`after_routes`+`check`; registered via `Hooks::initializers`. Only 2 core built-ins: `ExtraDbInitializer`, `MultiDbInitializer` (both with-db).
5. `doctor.rs`: `run_all` + 6 checks (DB, Queue, SeaOrmCLI, Deps, PublishedLocoVersion, per-initializer `check`); driven by `cargo loco doctor` with `--config`/`--production`.
6. `depcheck.rs` (own `VersionCheckError`/`Result`) reads `Cargo.lock`; min versions pin sea-orm/CLI to `2.0.0-rc` (still RC, not stable).
7. Error enum: `#[non_exhaustive]`, `Result<T,E=Error>` in `lib.rs:52`; 4 variants REMOVED (EnvVar, Hash, TaskJoinError, SemVer) — confirmed via commit `4a4a84ee`.
8. Misc: `data.rs` (`load_json_file[_sync]`, `LOCO_DATA_FOLDER` env), `tera::render_string`, `cargo_config::CargoConfig`.
9. TOP DOC GAPS: (a) NO dedicated testing page — only `models.md#testing`, and every example uses the non-compiling `boot_test::<App, Migrator>()` two-param form (STALE). (b) selector asserts, `request*` helpers, doctor check model, built-in initializers, `data`/`tera`/`cargo_config` are all MISSING.
10. 1.0 cleanup flags: leftover `println!("1"/"2"/"3")` debug in `extra_db.rs:19,25,30`; `default_scheme` in `RequestConfig` is never forwarded to axum-test's `TestServerConfig`.
