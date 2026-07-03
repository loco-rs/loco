# Inventory 01 — Core App Lifecycle & HTTP Layer

Verified against code at `release/0.17.0` (workspace 0.17.0, edition 2024, rust-version 1.94, Sea-ORM 2.0.0-rc, axum 0.8.1). All `file:line` refs are to `/Users/jondot/projects/loco/`.

Docs root: `docs-site/content/docs/`. Primary pages in this area: `the-app/controller.md`, `the-app/your-project.md`, `extras/pluggability.md`, `getting-started/guide.md`, `getting-started/axum-users.md`.

---

## 1. Application entrypoint / crate surface (`src/lib.rs`)

- **Purpose:** Crate root; re-exports public modules and the `Result`/`Error` aliases.
- **Public API:**
  - `pub use self::errors::Error;` (`src/lib.rs:5`)
  - `pub type Result<T, E = Error> = std::result::Result<T, E>;` (`src/lib.rs:52`)
  - Public modules: `bgworker, initializers, prelude, data, doctor, app, auth, boot, cache, config, controller, environment, errors, hash, logger, mailer, scheduler, task, storage, validation, cargo_config` (all `pub`); `db, model, schema` are `pub` only under `#[cfg(feature="with-db")]` (`src/lib.rs:16-21`); `cli` under `#[cfg(feature="cli")]` (`:28-29`); `testing`, `tests_cfg`, and `pub use axum_test::TestServer` under `#[cfg(feature="testing")]` (`:40-46`).
  - **Private modules** (not user-facing): `banner`, `depcheck`, `tera`, `env_vars` (`:7-32`).
- **1.0 note:** `#![doc = include_str!("../README.md")]` (`:3`) — the crate-level rustdoc IS the README; keep them in sync.

---

## 2. `Hooks` trait — user application contract (`src/app.rs:281-443`)

- **Purpose:** The single trait every Loco app implements (typically on `struct App`) to wire routes, workers, tasks, DB seed/truncate, and lifecycle callbacks. `#[async_trait]`, `Send`.
- **Required methods (no default — user MUST implement):**
  - `fn app_name() -> &'static str` (`:296`)
  - `async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult>` (`:323`)
  - `fn routes(_ctx: &AppContext) -> AppRoutes` (`:413`)
  - `async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()>` (`:422`)
  - `fn register_tasks(tasks: &mut Tasks)` (`:425`)
  - `#[cfg(feature="with-db")] async fn truncate(_ctx: &AppContext) -> Result<()>` (`:433`)
  - `#[cfg(feature="with-db")] async fn seed(_ctx: &AppContext, path: &Path) -> Result<()>` (`:437`)
- **Provided methods (overridable, have defaults):**
  - `fn app_version() -> String` → `"dev"` (`:285`)
  - `async fn serve(app, ctx, serve_params) -> Result<()>` — default binds `TcpListener`, `axum::serve` with `into_make_service_with_connect_info::<SocketAddr>()` and graceful shutdown calling `on_shutdown` (`:331-351`).
  - `fn init_logger(_ctx) -> Result<bool>` → `Ok(false)`; return `Ok(true)` to suppress Loco's logger and supply your own (`:360-362`).
  - `async fn load_config(env: &Environment) -> Result<Config>` → `env.load()` (`:368-370`).
  - `async fn before_routes(_ctx) -> Result<AxumRouter<AppContext>>` → empty router; the place to install a fallback handler before middleware (`:378`).
  - `async fn after_routes(router, _ctx) -> Result<AxumRouter>` (`:388`).
  - `async fn initializers(_ctx) -> Result<Vec<Box<dyn Initializer>>>` → `vec![]` (`:395`).
  - `fn middlewares(ctx) -> Vec<Box<dyn MiddlewareLayer>>` → `middleware::default_middleware_stack(ctx)` (`:401-403`).
  - `async fn before_run(_app_context) -> Result<()>` (`:408`).
  - `async fn after_context(ctx: AppContext) -> Result<AppContext>` (`:416`).
  - `async fn on_shutdown(_ctx)` → no-op (`:442`).
- **CLI/Generator:** `cargo loco generate` scaffolds `impl Hooks for App` in `src/app.rs` of a generated project.
- **DOC COVERAGE:** `extras/pluggability.md` covers hook-based extension points; `getting-started/axum-users.md` covers `before_routes`/`after_routes`/`middlewares`. Rate **THIN/STALE**: no single page enumerates the full Hooks surface. Concrete gaps:
  - `init_logger` (return `Ok(true)` to own the tracing stack) — **not documented anywhere** verified.
  - `load_config`, `after_context`, `before_run`, `app_version` — undocumented override points.
  - `serve` default now uses `into_make_service_with_connect_info::<SocketAddr>()` (required for `remote_ip`/connect-info) — verify docs don't show an outdated override.
- **1.0 note:** `boot` signature takes `environment: &Environment` (a reference to the enum, not `&str`). Some older docs/snippets show `environment: &str` — **flag any `&str` signature as STALE** (see `src/controller/mod.rs:47` doctest which correctly uses `&Environment`).

---

## 3. `AppContext` — shared application state (`src/app.rs:253-273`)

- **Purpose:** Cloneable, `FromRef`-enabled state handed to every handler/extractor via axum `State`.
- **Fields (all `pub`, users touch directly):**
  - `environment: Environment` (`:257`)
  - `db: DatabaseConnection` — `#[cfg(feature="with-db")]` (`:258-260`)
  - `queue_provider: Option<Arc<bgworker::Queue>>` (`:262`)
  - `config: Config` (`:264`)
  - `mailer: Option<EmailSender>` (`:266`)
  - `storage: Arc<Storage>` (`:268`)
  - `cache: Arc<cache::Cache>` (`:270`)
  - `shared_store: Arc<SharedStore>` (`:272`)
- **Derives:** `#[derive(Clone, FromRef)]` (`:253`) — `FromRef` auto-derives sub-state extraction so e.g. `State<DatabaseConnection>` works.
- **DOC COVERAGE:** `the-app/your-project.md` / `controller.md` mention `AppContext`. Rate **STALE**: `shared_store` field and `cache` field are newer; verify docs list all 8 fields. `db` being feature-gated is rarely noted.
- **1.0 note:** `db: DatabaseConnection` is Sea-ORM 2.0's connection type. i64-key / entity changes are downstream of this but the field type itself is unchanged.

---

## 4. `SharedStore` — type-keyed heterogeneous store (`src/app.rs:33-245`)

- **Purpose:** Concurrent `TypeId`-keyed DI container (backed by `DashMap`) for stashing arbitrary services on `AppContext`.
- **Public API:**
  - `SharedStore` struct `#[derive(Default, Debug)]` (`:34-38`)
  - `insert<T: 'static + Send + Sync>(&self, val: T)` (`:62`)
  - `remove<T>(&self) -> Option<T>` `#[must_use]` (`:103`)
  - `get_ref<T>(&self) -> Option<RefGuard<'_, T>>` (`:151`)
  - `get<T: ... + Clone>(&self) -> Option<T>` (`:195`)
  - `contains<T>(&self) -> bool` (`:221`)
  - `RefGuard<'a, T>` with `Deref<Target=T>` (`:228-245`)
- **Extractor:** `controller::extractor::shared_store::SharedStore<T>(pub T)` — `FromRequestParts<AppContext>`, requires `T: Any + Clone + Send + Sync`; returns `Error::InternalServerError` if missing (`src/controller/extractor/shared_store.rs:6-29`). Re-exported in prelude as `SharedStore` (`src/prelude.rs:26`).
- **NAMING HAZARD:** two distinct `SharedStore` types — `app::SharedStore` (the store) and the extractor `SharedStore<T>` — both reachable via `prelude`. Doc must disambiguate.
- **DOC COVERAGE:** Rate **MISSING** — no verified doc page for SharedStore DI. This is a real, tested feature (extensive unit tests `src/app.rs:480-712`).

---

## 5. `Initializer` trait (`src/app.rs:451-477`)

- **Purpose:** Pluggable one-time setup units kept in `src/initializers/`; can also register a doctor health check.
- **Public API:** `name(&self) -> String` (`:455`); `async before_run(&self, ctx)` (`:460`); `async after_routes(&self, router, ctx) -> Result<AxumRouter>` (`:467`); `async check(&self, ctx) -> Result<Option<crate::doctor::Check>>` (`:474`).
- **Wiring:** loaded via `Hooks::initializers`, run in `boot::run_app` (`before_run` at `src/boot.rs:452-454`) and `boot::setup_routes` (`after_routes` at `:525-527`).
- **DOC COVERAGE:** `extras/pluggability.md` covers initializers. Rate **THIN**: the `check()` doctor-integration method is newer — verify it's documented.

---

## 6. Boot / lifecycle (`src/boot.rs`)

- **Purpose:** Bootstraps context, DB, queue, routes, workers, scheduler; drives run modes and DB CLI commands.
- **Public types:**
  - `enum StartMode { ServerOnly, ServerAndWorker, ServerAndScheduler, WorkerOnly{tags:Vec<String>}, WorkerAndScheduler{tags:Vec<String>}, All }` (`:36-59`)
  - `struct BootResult { app_context, router: Option<Router>, worker: Option<Vec<String>>, run_scheduler: bool }` (`:61-70`)
  - `struct ServeParams { port: i32, binding: String }` (`:73-81`)
  - `enum RunDbCommand { Migrate, Down(u32), Reset, Status, Entities, Truncate, Seed{reset,from,dump,dump_tables}, Schema }` (`:270-294`)
  - `struct MiddlewareInfo { id, enabled, detail }` (`:581-585`)
- **Public fns:**
  - `create_context<H>(env, config) -> Result<AppContext>` (`:364`) — sets `RUST_BACKTRACE=1` if `logger.pretty_backtrace`; connects DB, mailer, queue, cache; calls `H::after_context`.
  - `create_app<H, M: MigratorTrait>(mode, env, config)` (with-db, `:409`) / `create_app<H>(...)` (no-db, `:425`) — context + `db::converge` + `bgworker::converge` + `run_app`.
  - `run_app<H>(mode, ctx) -> Result<BootResult>` (`:443`) — runs `before_run`, initializers, builds `BootResult` per mode.
  - `start<H>(boot, server_config, no_banner) -> Result<()>` (`:91`) — spawns scheduler, prints banner, serves and/or runs queue worker.
  - `run_task<H>(ctx, task, vars)` (`:188`), `run_scheduler<H>(...)` (`:250`), `run_db<H, M>(ctx, cmd)` (with-db, `:304`).
  - `list_endpoints<H>(ctx) -> Vec<ListRoutes>` (`:547`); `list_middlewares<H>(ctx) -> Vec<MiddlewareInfo>` (`:588`).
  - `shutdown_signal()` — Ctrl-C or SIGTERM (`:557`).
- **Config knobs:** `workers.mode == WorkerMode::BackgroundQueue` gates in-process queue worker (`:121,533`); `config.scheduler` / `SCHEDULER_CONFIG` env for scheduler (`:216,226`).
- **1.0 note:** `create_context` uses `unsafe { std::env::set_var(...) }` with a SAFETY comment — an edition-2024 change (`set_var` is now `unsafe`). `run_db` returns errors "mostly `sea_orm::DbErr`" — Sea-ORM 2.0 error type.
- **DOC COVERAGE:** `getting-started/guide.md` covers boot indirectly. Rate **THIN**: `StartMode` variants (esp. `ServerAndScheduler`, `WorkerAndScheduler`, tag filtering) and `SharedStore`/`after_context` boot flow under-documented.

---

## 7. Environment (`src/environment.rs`)

- **Purpose:** Selects config profile and loads `Config`.
- **Public API:** `enum Environment { Production, Development, Test, Any(String) }` (`:41-50`); `load(&self) -> Result<Config>` (`:59`); `load_from_folder(&self, path) -> Result<Config>` (`:72`); `resolve_from_env() -> String` (`:33`); `impl From<String>`, `Display`, `FromStr`.
- **Config knobs / env vars:** `LOCO_ENV`, then `RAILS_ENV`, then `NODE_ENV`, else `"development"` (`:21-24,33-38`); `CONFIG_FOLDER` env var overrides config directory (`:60-63` via `env_vars`).
- **DOC COVERAGE:** `getting-started/guide.md` (configuration). Rate **ACCURATE/THIN** — `RAILS_ENV`/`NODE_ENV` fallbacks and `CONFIG_FOLDER` rarely documented.

---

## 8. Logger (`src/logger.rs` + `src/config/logger.rs`)

- **Purpose:** Initializes the `tracing` stack (stdout + optional rolling file appender).
- **Public API:** `logger::init<H: Hooks>(config: &config::Logger) -> Result<()>` (`src/logger.rs:104`); enums `LogLevel {Off,Trace,Debug,Info(default),Warn,Error}` (`:16-36`), `Format {Compact(default),Pretty,Json}` (`:40-48`), `Rotation {Minutely,Hourly(default),Daily,Never}` (`:52-62`).
- **Filtering rules (`:84-99`):** 1) `RUST_LOG` wins; 2) else `logger.override_filter`; 3) else `MODULE_WHITELIST` (`loco_rs, sea_orm_migration, tower_http, sqlx::query, playground, loco_gen`, `:72-79`) + app crate, each at `config.level`.
- **Config knobs (`config/logger.rs`):** `logger.enable`, `pretty_backtrace`, `level`, `format`, `override_filter`, `file_appender{enable,non_blocking,level,format,rotation,dir,filename_prefix,filename_suffix,max_log_files}` (`:22-85`).
- **DOC COVERAGE:** `getting-started/guide.md` logging section. Rate **THIN/STALE**: `MODULE_WHITELIST` contents, `override_filter` semantics, and `file_appender` full knob list under-documented. `pretty_backtrace` runtime-cost warning (emitted at `boot.rs:373`) worth surfacing.

---

## 9. Banner (`src/banner.rs`)

- **Purpose:** Startup ASCII banner + runtime summary (env, db flags, logger, compilation mode, modes, serving line).
- **Public API:** `pub const BANNER: &str` (`:5`); `print_banner(boot_result, server_config)` (`:23`). Suppressed by `start(.., no_banner=true)`.
- **DOC COVERAGE:** Rate **MISSING** (cosmetic; low priority). Note: DB flag display (`enable_logging/auto_migrate/dangerously_recreate/dangerously_truncate`) is `#[cfg(with-db)]` only.

---

## 10. Controller module root (`src/controller/mod.rs`)

- **Purpose:** Error→HTTP response mapping, `Json` wrapper, error-constructor helpers, `ErrorDetail`.
- **Public API:**
  - `unauthorized<T,U>(msg) -> Result<U>` (`:112`), `bad_request<T,U>(msg) -> Result<U>` (`:121`), `not_found<T>() -> Result<T>` (`:130`).
  - `struct ErrorDetail { error: Option<String>, description: Option<String>, errors: Option<serde_json::Value> }` (`:133-142`); `ErrorDetail::new(error, description)` (`:147`), `with_reason(error)` (`:161`).
  - `struct Json<T>(pub T)` — `#[derive(FromRequest)] #[from_request(via(axum::Json), rejection(Error))]` + `IntoResponse` (`:170-178`).
  - `impl IntoResponse for Error` (`:180-253`) — the central error→status map.
  - Re-exports `AppRoutes, ListRoutes` (`:68`), `Routes` (`:75`).
- **Error→status map (verified, `:204-249`):** `NotFound`→404 `not_found`; `Unauthorized`→401 (logs warn); `CustomError(status,data)`→passthrough; `WithBacktrace`→prints red + backtrace, returns 400 "Bad Request"; `BadRequest`→400; `JsonRejection`→`err.status()`; `Validation`→400 with `errors` JSON; **everything else → 500 `internal_server_error`**.
- **DOC COVERAGE:** `the-app/controller.md` (errors/responses). Rate **THIN**: the exact status mapping and the `_ => 500` catch-all not spelled out. `CustomError`/`ErrorDetail` shape (public JSON body: `{error, description, errors}`) undocumented.

---

## 11. Routing — `AppRoutes` (`src/controller/app_routes.rs`) & `Routes` (`src/controller/routes.rs`)

- **Purpose:** Builder for the app's route tree; compiles to an axum `Router<AppContext>` then `Router` with state + middleware.
- **`AppRoutes` public API (`app_routes.rs`):** `with_default_routes()` (`:52`, adds `monitoring::routes()`), `empty()` (`:58`), `collect() -> Vec<ListRoutes>` (`:66`), `get_prefix()` (`:105`), `get_routes()` (`:111`), `prefix(&str)` (`:128`), `nest_prefix(&str)` (`:161`), `nest_route(prefix, Routes)` (`:190`), `nest_routes(prefix, Vec<Routes>)` (`:221`), `add_route(Routes)` (`:232`), `add_routes(Vec<Routes>)` (`:255`), `middlewares<H>(ctx)` (`:264`), `to_router<H>(ctx, app) -> Result<AXRouter>` (`:278`).
- **`ListRoutes` (`app_routes.rs:29-34`):** `{ uri: String, actions: Vec<axum::http::Method>, method: MethodRouter<AppContext> }`; `Display` prints `[GET,POST] /uri`.
- **`Routes` public API (`routes.rs`):** `new()` (`:25`), `at(prefix)` (`:53`), `add(uri, MethodRouter<AppContext>)` (`:81`), `merge(Routes)` (`:147`), `merge_all(Vec<Routes>)` (`:197`), `prefix(&str)` (`:228`), `layer<L>(layer)` (`:251`, per-route tower Layer), `nest(...)` (below `:273`). `Handler { uri, method, actions }` (`:15-20`).
- **Middleware ordering (`app_routes.rs:283-311`):** routes added first (onion core); middlewares applied via `mid.apply(app)` in `default_middleware_stack` order — LIFO at runtime (last in list = outermost = first to touch request). Well-commented; worth reproducing in docs.
- **URL normalization:** collapses `/+` → `/`, strips trailing slash, ensures leading slash (`app_routes.rs:80-91`).
- **CLI:** `cargo loco routes` → `list_endpoints` → `AppRoutes::collect`.
- **DOC COVERAGE:** `the-app/controller.md` covers routing, prefix, nesting. Rate **ACCURATE/THIN**: `merge`/`merge_all`, `Routes::layer`, `nest_route`/`nest_routes` newer builder methods — verify all are shown.

---

## 12. Format / response helpers (`src/controller/format.rs`)

- **Purpose:** Ergonomic response constructors + a chained `RenderBuilder`.
- **Free fns:** `empty()` (`:60`), `text(&str)` (`:81`), `json<T:Serialize>(t)` (`:110`), `empty_json()` (`:119`), `html(&str)` (`:139`), `yaml(&str)` (`:159`, `application/yaml`), `redirect(&str)` (`:182`), `view<V:ViewRenderer,S>(v,key,data)` (`:191`), `template<S>(tmpl,data)` (`:205`), `render() -> RenderBuilder` (`:405`).
- **`RenderBuilder` (`:212-402`):** `new`/`default`, `response()` (escape to axum `Builder`), `status<T>`, `header<K,V>`, `etag(&str)`, `cookies(&[Cookie])`, `text`, `empty`, `view`, `template`, `html`, `json<T>`, `redirect` (303 SEE_OTHER + Location), `redirect_with_header_key<K>` (e.g. HX-Redirect).
- **Re-export:** `format` in prelude (`src/prelude.rs:35`).
- **DOC COVERAGE:** `the-app/controller.md` shows `format::json`, `render().etag()`. Rate **THIN**: `yaml`, `empty_json`, `redirect_with_header_key`, `cookies`, `RenderBuilder::response` escape-hatch undocumented.

---

## 13. Monitoring / health routes (`src/controller/monitoring.rs`)

- **Purpose:** Built-in liveness/readiness endpoints, auto-mounted by `with_default_routes()`.
- **Endpoints:** `GET /_ping` (`ping`, `:27`), `GET /_health` (`health`, `:35`), `GET /_readiness` (`readiness`, `:44`). `struct Health { ok: bool }` (`:18-21`). `routes()` (`:101`).
- **Readiness checks (`:44-98`):** pings DB (`#[cfg(with-db)]`), queue provider (if present), and cache (`#[cfg(cache_inmem|cache_redis)]`, matches `CacheConfig::{InMem,Redis,Null}`). Any failure → 503 `{ok:false}`; else 200 `{ok:true}`.
- **DOC COVERAGE:** Rate **THIN/MISSING**: the three underscore-prefixed endpoints and 503-on-dependency-failure semantics are barely documented. Feature-gating of the checks worth noting.

---

## 14. Describe / route introspection (`src/controller/describe.rs`)

- **Purpose:** Recovers HTTP methods from an axum `MethodRouter` (axum doesn't expose them) by regex on its Debug string.
- **Public API:** `method_action(method: &MethodRouter<AppContext>) -> Vec<http::Method>` (`:19`). Regex `\b(\w+):\s*(BoxedHandler|Route)\b` (`:11`).
- **1.0 note / FRAGILITY:** depends on axum 0.8's `Debug` format of `MethodRouter`. An axum upgrade can silently break `cargo loco routes` method display. Flag as a maintenance risk.
- **DOC COVERAGE:** Internal; **MISSING** by design (not user-facing).

---

## 15. Backtrace printing (`src/controller/backtrace.rs`)

- **Purpose:** Pretty-prints filtered backtraces for `Error::WithBacktrace` (used by the error IntoResponse).
- **Public API:** `print_backtrace(bt: &std::backtrace::Backtrace) -> Result<()>` (`:59`). Name blocklist + file blocklist regexes filter framework frames (`:10-57`).
- **Config knob:** activated by `logger.pretty_backtrace` (sets `RUST_BACKTRACE=1` in `boot::create_context`).
- **DOC COVERAGE:** `getting-started/guide.md` may mention pretty backtraces. Rate **THIN**.

---

## 16. Middleware layer — trait + FULL stack (`src/controller/middleware/`)

- **Trait `MiddlewareLayer` (`mod.rs:46-72`):** `name(&self) -> &'static str`; `is_enabled(&self) -> bool` (default `true`); `config(&self) -> serde_json::Result<Value>`; `apply(&self, AXRouter<AppContext>) -> Result<AXRouter<AppContext>>`.
- **`default_middleware_stack(ctx)` (`mod.rs:76-171`)** and **`server.middlewares` config struct `Config` (`mod.rs:174-212`)**.
- **FULL middleware set** (name / config key / struct / file / default-enabled):
  1. **limit_payload** — `limit_payload::LimitPayload` (`limit_payload.rs:26`, key `limit_payload:66`). Knobs: `body_limit: DefaultBodyLimitKind` (enum `:20`, e.g. `"5mb"` / disabled). Default enabled (`unwrap_or_default`).
  2. **cors** — `cors::Cors` (`cors.rs:19`). Knobs: `enable, allow_origins, allow_headers, allow_methods, expose_headers, allow_credentials, max_age, vary` (`:20-41`); `cors()` builds `CorsLayer` (`:90`). Default **disabled**.
  3. **catch_panic** — `catch_panic::CatchPanic { enable }` (`catch_panic.rs:19`). Default **enabled**.
  4. **etag** — `etag::Etag { enable }` (`etag.rs:28`). Default **enabled**.
  5. **remote_ip** — `remote_ip::RemoteIpMiddleware { enable, trusted_proxies: Option<Vec<String>> }` (`remote_ip.rs:96`). Default **disabled**. Also exposes extractor `RemoteIP` enum (`:181`) + `RemoteIPMiddleware<S>` service (`:256`).
  6. **compression** — `compression::Compression { enable }` (`compression.rs:15`). Default **disabled**.
  7. **timeout_request** — `timeout::TimeOut { enable, timeout: u64 }` (`timeout.rs:24`; key `timeout_request:45`). Default **disabled**.
  8. **static** — `static_assets::StaticAssets { enable, must_exist, folder{uri,path}, fallback, precompressed, cache_control }` (`static_assets.rs:25`; key `static:82`). Default **disabled**. Under `#[cfg(embedded_assets)]` swapped for `static_assets_embedded` (`mod.rs:21-27`).
  9. **secure_headers** — `secure_headers::SecureHeader { enable, preset: String, overrides: Option<BTreeMap<String,String>> }` (`secure_headers.rs:79`). Default **disabled**. Presets from `secure_headers.json`.
  10. **logger** — `logger::Config { enable }` → `logger::Middleware` via `logger::new(config, env)` (`logger.rs:22,37`). Default **enabled**.
  11. **request_id** — `request_id::RequestId { enable }` (`request_id.rs:29`); exposes `LocoRequestId(String)` w/ `.get()` (`:64-69`). Default **enabled**.
  12. **fallback** — `fallback::Fallback { enable, code: StatusCode, file: Option<String>, not_found: Option<String> }` (`fallback.rs:18`); `StatusCodeWrapper(pub StatusCode)` (`:15`). Default enabled **only when `environment != Production`** (`mod.rs:164`). Ships `fallback.html`.
  13. **powered_by** — `powered_by::Middleware` via `powered_by::new(ctx.config.server.ident.as_deref())` (`powered_by.rs:28,35`; key `powered_by:63`). Sets `Server` header; not toggled by `enable` — controlled by `server.ident`.
- **Config knob:** all under `server.middlewares.<name>` (`config/server.rs:44`).
- **CLI:** `cargo loco middleware --config` → `list_middlewares` (`boot.rs:588`).
- **1.0 note:** middleware `Config` struct field for static is `#[serde(rename="static")]` (`mod.rs:198`).
- **DOC COVERAGE:** `the-app/controller.md:326-439` documents middleware well, including `cargo loco middleware --config` sample output. Rate **ACCURATE but verify sample output**: the doc's printed JSON (e.g. `"expose_header":[""]`) must match current `Cors` serialization (`expose_headers` field is now `Vec<String>` defaulting via `default_expose_headers`). **DISCREPANCY to check:** doc shows `expose_header` (singular) at `controller.md:348,429` while the struct field is `expose_headers` (`cors.rs:32`) — likely STALE sample output. Also verify `secure_headers` `preset`/`overrides` knobs are documented (currently thin).

---

## 17. Extractors — FULL set (`src/controller/extractor/`)

- **`format::Format(pub RespondTo)` + `RespondTo {None,Html,Json,Xml,Other(String)}`** (`middleware/format.rs:14-22`) — content negotiation from `Content-Type`/`Accept`; both impl `FromRequestParts`. `get_respond_to(headers)` (`:40`). Re-exported in prelude (`prelude.rs:37`).
- **`remote_ip::RemoteIP` enum** (`middleware/remote_ip.rs:181`) — extracts computed client IP; re-exported in prelude (`prelude.rs:38`).
- **`shared_store::SharedStore<T>(pub T)`** — DI extractor (`extractor/shared_store.rs:6`), prelude (`prelude.rs:26`).
- **Validation extractors (`extractor/validate.rs`), all `FromRequest`, newtypes `(pub T)`:**
  - `JsonValidateWithMessage<T>` (`:41`), `FormValidateWithMessage<T>` (`:86`), `JsonValidate<T>` (`:132`), `FormValidate<T>` (`:180`), `QueryValidateWithMessage<T>` (`:228`), `QueryValidate<T>` (`:276`). `JsonValidate`/`JsonValidateWithMessage` re-exported in prelude (`prelude.rs:27`).
- **Auth extractors (`extractor/auth.rs`, `#[cfg(feature="auth_jwt")]` via prelude `prelude.rs:23-24`):**
  - `JWTWithUser<T: Authenticable> { claims: UserClaims, user: T }` (`:52`, `#[cfg(with-db)]`, `FromRequestParts`).
  - `JWT { claims: UserClaims }` (`:105`, `FromRequestParts`).
  - `ApiToken<T: Authenticable>` (`:257`, `FromRequestParts`).
  - Helper fns: `extract_jwt_from_request_parts` (`:126`), `get_jwt_from_config` (`:152`), `extract_token` (`:167`), `extract_token_from_header` (`:208`), `extract_token_from_cookie` (`:224`), `extract_token_from_query` (`:239`). Constants `TOKEN_PREFIX="Bearer "`, `AUTH_HEADER="authorization"` (`:45-46`).
  - `UserClaims { pid: String, claims: Map<String,Value> }` (`auth/jwt.rs:18`).
- **View engine extractor:** `views::ViewEngine<E>(pub E)` impl `FromRequestParts` (needs `TeraLayer` Extension installed; panics if missing) (`views/mod.rs:69-96`).
- **Config for auth extractors:** `auth.jwt.{secret, expiration, location}`; `JWTLocation {Bearer, Query{name}, Cookie{name}}` and `JWTLocationConfig::{Single, Multiple}` (`config/auth.rs:21-54`). Extractor imports config `JWT` as `JWTConfig` (`extractor/auth.rs:33`).
- **Feature flags:** `auth_jwt` (default-on; now selects `jsonwebtoken/rust_crypto`, Cargo.toml `:41`) gates all JWT extractors; `JWTWithUser`/`ApiToken` additionally need `with-db`.
- **DOC COVERAGE:** `extras/authentication.md` covers JWT extractors; `controller.md` covers validation. Rate **THIN/STALE**:
  - Multi-location JWT (`JWTLocationConfig::Multiple`, tried in order) is newer — verify documented.
  - `QueryValidate`/`QueryValidateWithMessage` and the `*WithMessage` variants may be underdocumented.
  - `RespondTo::Xml`/`Other` and full negotiation table undocumented.
  - **1.0 note:** `auth_jwt` no longer bundles a crypto backend (jsonwebtoken 10) — any doc saying it's zero-config C-free should be re-verified against the `rust_crypto` selection.

---

## 18. Views (`src/controller/views/`)

- **Purpose:** Tera-based server-side rendering; pluggable `ViewRenderer`.
- **Public API:** `trait ViewRenderer { render<S:Serialize>(&self, key, data) -> Result<String> }` (`views/mod.rs:20`); `ViewEngine<E>(pub E)` + `new`/`From`/`FromRequestParts` (`:29-96`); free `template<S>(tmpl, data) -> Result<String>` (`:55`).
- **`TeraView` (`views/engine.rs`):** `pub static DEFAULT_ASSET_FOLDER = "assets"` (`:13`); `TeraView` struct (`:25`); `build()` → `assets/views` (`:36`); `build_with_post_process(...)` (`:49`); `from_custom_dir<P>(path, post)` (`:83`); `impl ViewRenderer` (`:176`). Under `#[cfg(embedded_assets)]` swapped for `engine_embedded` (`views/mod.rs:1-10`).
- **Also:** `views::pagination` (`#[cfg(with-db)]`), `views::tera_builtins`. Prelude re-exports `TeraView, ViewEngine, ViewRenderer` (`prelude.rs:41`).
- **DOC COVERAGE:** `the-app/views.md`. Rate **THIN/STALE**: `embedded_assets` feature (embeds templates+assets into binary, swaps engine) is a significant 1.0-era capability — verify `views.md` covers it. `build_with_post_process` undocumented.

---

## 19. Prelude (`src/prelude.rs`)

- **Purpose:** One-import surface (`use loco_rs::prelude::*`) for app code.
- **Notable re-exports:** axum `debug_handler`, extractors `Form/Multipart/Path/Query/State`, routing verbs `get/post/put/delete/head/options/patch/trace` (`:2-7`); `serde_json::json as data` (`:21`); Loco `AppContext, Initializer, BackgroundWorker, Queue, format, middleware::{Format,RespondTo,RemoteIP}, not_found/unauthorized/bad_request, views::{TeraView,ViewEngine,ViewRenderer}, Json, Routes, Error, mailer::Mailer, Task/TaskInfo, validation::{Validatable,ValidatorTrait}, Result` (`:23-51`). `#[cfg(with-db)]` adds sea-orm traits + `model::{query,Authenticable,ModelError,ModelResult}`.
- **DOC COVERAGE:** `getting-started/axum-users.md` mentions prelude. Rate **THIN** — no exhaustive prelude reference.
- **1.0 note:** Sea-orm prelude re-exports (`ActiveModelTrait, EntityTrait, Set, ...`, `Date/DateTimeUtc/Decimal/Uuid`) come from Sea-ORM 2.0 — verify names unchanged post-2.0 (esp. `ActiveValue`, `IntoActiveModel`).

---

## Cross-cutting 1.0 / Sea-ORM 2.0 / edition-2024 notes

- **edition 2024 / rustc 1.94:** `std::env::set_var` is now `unsafe` — appears at `boot.rs:372` and `environment.rs` tests with SAFETY comments. Any doc code snippet calling `set_var` must add `unsafe {}`.
- **Sea-ORM 2.0.0-rc** (`Cargo.toml:73`): `AppContext.db: DatabaseConnection`, `Error::DB(#[from] sea_orm::DbErr)` (`errors.rs:86`), `run_db` commands are all downstream. i64-key changes are in the model/schema area (not this file set) but the prelude re-exports the entity traits.
- **Error enum narrowing (`errors.rs:30-151`):** `#[non_exhaustive]` enum. Public HTTP-facing variants users construct: `Message, Unauthorized, NotFound, BadRequest, CustomError(StatusCode, ErrorDetail), InternalServerError`. Feature-gated variants: `DB`/`Model` (`with-db`), `Redis` (`bg_redis`), `Sqlx` (`bg_pg`|`bg_sqlt`), `Generators` (`debug_assertions`). Constructors: `wrap`, `msg`, `string`, `bt` (backtrace capture only when `RUST_BACKTRACE=1`). `serde_json::Error` → `Error::JSON(..).bt()` (`:24-28`).
- **auth_jwt crypto backend** (`Cargo.toml:37-41`): jsonwebtoken 10 → `rust_crypto` explicitly selected; onboarding docs claiming "no crypto config" should be re-verified.
- **embedded_assets feature** swaps both the static-assets middleware and the Tera view engine to embedded variants — a doc-worthy build-time toggle currently thinly covered.

## Top doc gaps/inaccuracies (ranked)
1. **SharedStore DI (store + extractor)** — MISSING; fully implemented & tested.
2. **Hooks full surface** — `init_logger`, `load_config`, `after_context`, `before_run`, `app_version` undocumented; some snippets use stale `environment: &str` instead of `&Environment`.
3. **`cargo loco middleware --config` sample in controller.md** — likely STALE: shows `expose_header` (singular) vs struct `expose_headers`.
4. **Monitoring endpoints** `/_ping /_health /_readiness` + 503-on-dependency-failure — THIN/MISSING.
5. **Error→HTTP status map** and `ErrorDetail` JSON body shape — THIN.
6. **JWT multi-location** (`JWTLocationConfig::Multiple`) and `auth_jwt` rust_crypto change — STALE/THIN.
7. **embedded_assets** (view engine + static swap) — THIN.
8. **describe.rs regex fragility** vs axum 0.8 Debug format — maintenance risk to note.
