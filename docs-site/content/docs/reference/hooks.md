+++
title = "Hooks trait"
description = "The complete Hooks trait surface: every required and provided method, its signature, and when it runs."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 7
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

`Hooks` (`#[async_trait]`, `Send`, `src/app.rs:281-443`) is the single trait every Loco application implements — typically on a `struct App` in `src/app.rs` — to wire routing, workers, tasks, database seed/truncate, and lifecycle callbacks. `cargo loco generate` scaffolds an `impl Hooks for App` for you; this page is the exhaustive reference for what that `impl` can and must contain.

## Required methods

No default implementation. The trait will not compile without these.

| Method | Signature | Purpose |
|---|---|---|
| `app_name` | `fn app_name() -> &'static str` (`:296`) | Returns the app's crate name (conventionally `env!("CARGO_CRATE_NAME")`). |
| `boot` | `async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult>` (`:323`) | Initializes and boots the application for the given `StartMode` and `Environment`. Typically delegates to `create_app::<Self, Migrator>(mode, environment, config)` (with DB) or `create_app::<Self>(mode, environment, config)` (without DB). |
| `routes` | `fn routes(_ctx: &AppContext) -> AppRoutes` (`:413`) | Defines the application's routing configuration. |
| `connect_workers` | `async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()>` (`:422`) | Registers background-job workers against the provided `Queue`. |
| `register_tasks` | `fn register_tasks(tasks: &mut Tasks)` (`:425`) | Registers custom `cargo loco task` entries with the `Tasks` registry. |
| `truncate` | `#[cfg(feature = "with-db")] async fn truncate(_ctx: &AppContext) -> Result<()>` (`:433`) | Truncates application tables. Invoked when `config.database.dangerously_truncate` is `true`; useful before tests. |
| `seed` | `#[cfg(feature = "with-db")] async fn seed(_ctx: &AppContext, path: &Path) -> Result<()>` (`:437`) | Seeds the database with initial data from `path`. |

`truncate` and `seed` only exist on the trait when the `with-db` Cargo feature is enabled.

## Provided methods

Have a default implementation; override to change behavior.

| Method | Signature | Default behavior |
|---|---|---|
| `app_version` | `fn app_version() -> String` (`:285`) | Returns `"dev".to_string()`. |
| `serve` | `async fn serve(app: AxumRouter, ctx: &AppContext, serve_params: &ServeParams) -> Result<()>` (`:331-351`) | Binds a `tokio::net::TcpListener` on `serve_params.binding:serve_params.port` and runs `axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())` with graceful shutdown; on shutdown, calls `Self::on_shutdown(&ctx)`. |
| `init_logger` | `fn init_logger(_ctx: &AppContext) -> Result<bool>` (`:360-362`) | Returns `Ok(false)`, meaning Loco initializes its own tracing/logging stack. |
| `load_config` | `async fn load_config(env: &Environment) -> Result<Config>` (`:368-370`) | Returns `env.load()` — the standard `config/{env}.yaml` (+ `.local.yaml` overlay) loading path. |
| `before_routes` | `async fn before_routes(_ctx: &AppContext) -> Result<AxumRouter<AppContext>>` (`:378`) | Returns `Ok(AxumRouter::new())` — an empty router. |
| `after_routes` | `async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter>` (`:388`) | Returns `Ok(router)` unchanged. |
| `initializers` | `async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>>` (`:395`) | Returns `Ok(vec![])` — no initializers. |
| `middlewares` | `fn middlewares(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>>` (`:401-403`) | Returns `middleware::default_middleware_stack(ctx)`. |
| `before_run` | `async fn before_run(_app_context: &AppContext) -> Result<()>` (`:408`) | Returns `Ok(())` — no-op. |
| `after_context` | `async fn after_context(ctx: AppContext) -> Result<AppContext>` (`:416`) | Returns `Ok(ctx)` unchanged. |
| `on_shutdown` | `async fn on_shutdown(_ctx: &AppContext)` (`:442`) | No-op. |

## Override points

The methods below are the least-documented parts of `Hooks`. Each entry states exactly what overriding changes.

### `init_logger` — own your tracing stack

```rust
fn init_logger(_ctx: &AppContext) -> Result<bool>
```

Runs once during boot, before the rest of the app context is wired up. Returning `Ok(true)` tells Loco **not** to initialize its own logger — the app is then responsible for setting up a complete tracing/logging stack itself. Returning `Ok(false)` (the default) leaves Loco's built-in logger in place.

### `load_config` — replace the config loader

```rust
async fn load_config(env: &Environment) -> Result<Config>
```

Runs during boot to produce the `Config` passed into `boot`. The default is `env.load()` (the standard `config/{env}.yaml` file resolution). Override to load configuration from a different source (e.g. a remote config service) while still returning a `Config`.

### `after_context` — post-process `AppContext`

```rust
async fn after_context(ctx: AppContext) -> Result<AppContext>
```

Runs after `AppContext` has been fully constructed (db, cache, storage, mailer, queue provider all present) but before routes are built. Takes `ctx` by value and must return a (possibly modified) `AppContext` — the only hook that lets you replace fields on the context itself.

### `before_run` — pre-run resource loading

```rust
async fn before_run(_app_context: &AppContext) -> Result<()>
```

Runs before the app starts serving/running (applies to the server and to other run modes such as tasks/jobs, not only HTTP serve). Use it to load or warm resources that don't belong on `AppContext` itself.

### `serve` — the HTTP serve loop

```rust
async fn serve(app: AxumRouter, ctx: &AppContext, serve_params: &ServeParams) -> Result<()>
```

Runs when the app is started in server mode. The default binds a `TcpListener` and calls `axum::serve` with `app.into_make_service_with_connect_info::<SocketAddr>()` — the `connect_info` layer is required for `remote_ip`/client-address extraction in controllers — wrapped in graceful shutdown that calls `on_shutdown`. Override only to change the transport/serve mechanics (e.g. custom TLS termination); overriding without preserving `into_make_service_with_connect_info` will break connect-info extraction.

### `app_version` — composite version string

```rust
fn app_version() -> String
```

Called wherever Loco reports its version (e.g. `cargo loco version`, `/_ping`/`/_health` style diagnostics). Default is the literal `"dev"`; override to compose a real version string, e.g. from `CARGO_PKG_VERSION` plus a git SHA.

## `boot` signature note

`boot`'s second parameter is `environment: &Environment` — a reference to the `Environment` enum, **not** `&str`:

```rust
async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult>
```

Some existing docs and snippets show `environment: &str`; that signature is stale (the rustdoc example inside `src/app.rs:308` and `:315` itself still shows `&str` and should not be copied). `src/controller/mod.rs:47` is a correct reference example using `&Environment`.
