---
title: AppContext & prelude
description: "The AppContext struct field-by-field, and everything use loco_rs::prelude::* brings into scope."
sidebar:
  order: 6
---

`AppContext` is the cloneable, `axum`-`State`-compatible struct that carries every shared resource of a Loco app (DB connection, cache, queue, mailer, storage, config, and an open-ended DI slot). `loco_rs::prelude` is the single-import surface app code pulls in instead of naming individual `loco_rs` and `axum` paths. Both are declared in `src/app.rs` and `src/prelude.rs`.

## `AppContext`

Defined at `src/app.rs:253-273`:

```rust
#[derive(Clone, FromRef)]
#[non_exhaustive]
pub struct AppContext {
    pub environment: Environment,
    #[cfg(feature = "with-db")]
    pub db: DatabaseConnection,
    pub queue_provider: Option<Arc<bgworker::Queue>>,
    pub config: Config,
    pub mailer: Option<EmailSender>,
    pub storage: Arc<Storage>,
    pub cache: Arc<cache::Cache>,
    pub shared_store: Arc<SharedStore>,
}
```

### Derives

`#[derive(Clone, FromRef)]` (`src/app.rs:253`). `FromRef` (from `axum`) auto-generates `impl FromRef<AppContext> for <FieldType>` for each field, so a handler can extract a single field directly — e.g. `State<DatabaseConnection>` or `State<Arc<cache::Cache>>` — instead of always taking the whole `State<AppContext>`.

### Fields

| Field | Type | Feature gate | Purpose |
|---|---|---|---|
| `environment` | `Environment` | none | Which profile the app booted under (`Production` / `Development` / `Test` / `Any(String)`); drives config-file selection. |
| `db` | `DatabaseConnection` (Sea-ORM 2.0) | `#[cfg(feature = "with-db")]` | The pooled Sea-ORM database connection used by every entity query and by `db::converge` migrations. Absent entirely from the struct when `with-db` is off. |
| `queue_provider` | `Option<Arc<bgworker::Queue>>` | none | The background-job queue (Redis / Postgres / SQLite / in-process), if the app was booted with one wired up. `None` for queue-less apps. |
| `config` | `Config` | none | The fully loaded, deserialized `config/<environment>.yaml` (+ `.local.yaml` overlay). |
| `mailer` | `Option<EmailSender>` | none | The configured email-sending backend (SMTP or stub), if the app enabled one. |
| `storage` | `Arc<Storage>` | none | The file/object storage abstraction (local disk or a cloud backend selected by the `storage_*` feature flags). |
| `cache` | `Arc<cache::Cache>` | none | The cache handle (in-memory, Redis, or null backend per `cache_*` flags / `CacheConfig`). |
| `shared_store` | `Arc<SharedStore>` | none | A `TypeId`-keyed, concurrent DI container (backed by `DashMap`) for stashing arbitrary app-defined services — see below. |

`db` is the only field that is compiled out (not just `None`-able) when its feature (`with-db`) is disabled — every other field is unconditionally present, with `Option`/empty-default standing in for "not configured."

### Constructing one: `builder` / `into_builder`

`AppContext` is `#[non_exhaustive]` (`src/app.rs:255`), so app code cannot write a struct literal for it and cannot use functional-update syntax (`AppContext { storage, ..ctx }`). Adding a field in a later release is therefore not a breaking change. Two constructors take its place (`src/app.rs:284-413`):

- `AppContext::builder(environment, db, config)` — or `builder(environment, config)` without `with-db` — returns an `AppContextBuilder`. The required components are arguments; `queue_provider`, `mailer`, `storage`, `cache` and `shared_store` are setter methods, and `build()` fills any of them left unset with a no-op default (null storage driver, null cache, empty shared store).
- `ctx.into_builder()` turns an existing context back into a builder carrying **every** component over. This is what [`Hooks::after_context`](/docs/reference/hooks) should use: starting fresh from `AppContext::builder` compiles, but silently drops whatever boot already put on the context (the mailer, the queue provider, the cache, the shared store), whereas round-tripping replaces one component and keeps the rest.

```rust
async fn after_context(ctx: AppContext) -> Result<AppContext> {
    Ok(ctx
        .into_builder()
        .storage(Storage::single(storage::drivers::local::new()).into())
        .build())
}
```

### `SharedStore` — the generic DI slot

`shared_store: Arc<SharedStore>` (`src/app.rs:33-245, 272`) is a small heterogeneous store for services that don't have a dedicated `AppContext` field. Its API:

- `insert<T: 'static + Send + Sync>(&self, val: T)` (`:62`)
- `remove<T>(&self) -> Option<T>` (`:103`)
- `get_ref<T>(&self) -> Option<RefGuard<'_, T>>` (`:151`) — borrowed access via a `Deref<Target = T>` guard
- `get<T: Clone + 'static + Send + Sync>(&self) -> Option<T>` (`:195`) — cloning access
- `contains<T>(&self) -> bool` (`:221`)

To read a stashed value from inside a handler, use the extractor of the same name: `controller::extractor::shared_store::SharedStore<T>(pub T)`, which implements `FromRequestParts<AppContext>` and returns `Error::InternalServerError` if `T` was never inserted (`src/controller/extractor/shared_store.rs:6-29`).

**Naming note:** `app::SharedStore` (the container held on `AppContext`) and the extractor `controller::extractor::shared_store::SharedStore<T>` (the axum extractor) are two distinct types that share a name. Both are reachable through the prelude — see below — so disambiguate by context: the field type is the store, the tuple-struct-with-generic is the extractor.

## `loco_rs::prelude`

`use loco_rs::prelude::*;` (`src/prelude.rs`) is the standard single import for app code (controllers, models, workers, tasks). It re-exports, unconditionally unless noted:

**Async / axum plumbing**
- `async_trait::async_trait`
- `axum::debug_handler`
- `axum::extract::{Form, Multipart, Path, Query, State}`
- `axum::response::{IntoResponse, Response}`
- `axum::routing::{delete, get, head, options, patch, post, put, trace}`
- `axum_extra::extract::cookie`

**Third-party helpers**
- `chrono::NaiveDateTime as DateTime`
- `include_dir::{include_dir, Dir}`
- `serde_json::json as data` — sugar so controller/view code can write `data!({"item": ..})` instead of `json!`
- `validator::Validate`

**Core Loco types**
- `app::{AppContext, Initializer}`
- `bgworker::{BackgroundWorker, Queue}`
- `errors::Error` and `Result` (the crate's `Result<T, Error>` alias)
- `mailer` (the module itself) and `mailer::Mailer`
- `task::{self, Task, TaskInfo}`
- `validation::{self, Validatable, ValidatorTrait}`

**Controller layer**
- `controller::{bad_request, not_found, unauthorized}` — error-response constructor fns
- `controller::format` — the response-builder module (`format::json`, `format::render()`, ...)
- `controller::middleware::format::{Format, RespondTo}` — content-negotiation extractor/enum
- `controller::middleware::remote_ip::RemoteIP` — computed-client-IP extractor
- `controller::middleware::MiddlewareStackExt` — the `insert_before` / `insert_after` / `replace` / `delete` helpers for editing the default middleware stack inside `Hooks::middlewares` (see the [middleware catalog](/docs/reference/middleware))
- `controller::extractor::shared_store::SharedStore` — the DI extractor (see above)
- `controller::extractor::validate::{JsonValidate, JsonValidateWithMessage}` — validating-body extractors
- `controller::views::{engines::TeraView, ViewEngine, ViewRenderer}`
- `controller::{Json, Routes}`

**Feature-gated**
- `#[cfg(feature = "auth")] controller::extractor::auth` (the whole `auth` extractor module: `JWT`, `JWTWithUser`, `ApiToken`, `UserClaims`, token-extraction helpers)
- `#[cfg(feature = "with-db")]`:
  - Sea-ORM traits and types: `ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, Set, TransactionTrait`
  - Sea-ORM scalar re-exports: `sea_orm::prelude::{Date, DateTimeUtc, DateTimeWithTimeZone, Decimal, Uuid}`
  - `model::{query, Authenticable, ModelError, ModelResult}` — plus a nested `pub mod model { pub use crate::model::query; }`, so `query` is reachable both as `loco_rs::prelude::query` and as `loco_rs::prelude::model::query`
- `#[cfg(feature = "multi-tenancy")] model::{TenantEntity, TenantQueryExt, TenantActiveModelExt}` — opt-in tenant scoping helpers; this feature also enables `with-db`
- `#[cfg(feature = "testing")] crate::testing::prelude::*` — pulled in only for apps/tests built with the `testing` feature

Source: `src/prelude.rs` (verified against `HEAD` at authoring time).
