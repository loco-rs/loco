---
title: Error model
description: The loco-rs Error enum, its HTTP status mapping, the ErrorDetail JSON body, and the wrap/msg/string/bt constructors.
sidebar:
  order: 8
---

`loco-rs` uses a single crate-wide error type. This page documents its shape, how it becomes an HTTP response, and the helpers for constructing it.

## `Result` and `Error`

- `pub type Result<T, E = Error> = std::result::Result<T, E>` (`src/lib.rs:52`).
- `pub use self::errors::Error` (`src/lib.rs:5`) — re-exported at the crate root and in `loco_rs::prelude`.
- `Error` is declared `#[non_exhaustive]` (`src/errors.rs:31`). Any `match Error { .. }` outside the crate **must** include a wildcard `_ =>` arm — the compiler enforces this. Inside `loco_rs` itself `#[non_exhaustive]` does not apply, and the status map below deliberately does not use a wildcard.

## Variant → HTTP status map

`impl IntoResponse for Error` (`src/controller/mod.rs:198`) matches on the error and produces `(StatusCode, ErrorDetail)`, then serializes `ErrorDetail` as the JSON response body. The match is **exhaustive on purpose — there is no `_ =>` arm** (`src/controller/mod.rs:274-284`): every internal/infrastructure variant is listed by name in the arm that maps to 500, so adding a new `Error` variant anywhere in the crate is a compile error here until someone decides what status it should carry, rather than silently becoming a 500.

| Variant | Produced when | HTTP status | Response `ErrorDetail` |
|---|---|---|---|
| `NotFound` | Returned by the `not_found()` helper, or directly. | 404 | `{"error":"not_found","description":"Resource was not found"}` |
| `Unauthorized(String)` | Returned by the `unauthorized(msg)` helper, or directly. Also logs `tracing::warn!(err)` with the original message (the message itself is **not** sent to the client). | 401 | `{"error":"unauthorized","description":"You do not have permission to access this resource"}` |
| `CustomError(StatusCode, ErrorDetail)` | Constructed directly when the caller wants an arbitrary status and body. | passthrough — whatever `StatusCode` was supplied | passthrough — whatever `ErrorDetail` was supplied |
| `WithBacktrace { inner, backtrace }` | Wraps another error variant; produced by calling `.bt()` on an `Error` (backtrace is only captured when `RUST_BACKTRACE` is set — see Constructors below). Also prints the inner error (red, underlined) and the filtered backtrace to stdout via `backtrace::print_backtrace`. | passthrough — the wrapped error's own status | passthrough — the wrapped error's own body. `mod.rs:238-245` re-enters `into_response` on `*inner`, so capturing a backtrace never changes the HTTP result (an internal error stays 500 rather than being flattened to 400) |
| `BadRequest(String)` | Returned by the `bad_request(msg)` helper, or directly. | 400 | `{"error":"Bad Request","description":"<msg>"}` |
| `JsonRejection(JsonRejection)` | Axum's `Json` extractor rejects a malformed/missing request body (surfaced via the `Json<T>` wrapper's `#[from_request(rejection(Error))]`). Logs `tracing::debug!(err = err.body_text(), ...)`. | `err.status()` — axum's own rejection status (commonly 400/415/422) | `{"error":"Bad Request"}` |
| `AxumFormRejection(FormRejection)` | Axum's `Form` extractor rejects a malformed request body. Logs `tracing::debug!(err = err.body_text(), ...)` (`mod.rs:254-257`). | `err.status()` — axum's own rejection status | `{"error":"Bad Request"}` |
| `Validation(ModelValidationErrors)` | A `validator`-crate validation failure converted `#[from] ModelValidationErrors`. | 400 | `{"errors": <serde_json::Value of the field errors>}` — note `error`/`description` are `None` here; only `errors` is populated |
| `Model(ModelError::EntityNotFound)` | A model lookup found no row (`with-db`, `mod.rs:261-265`). | 404 | `{"error":"not_found","description":"Resource was not found"}` |
| `Model(ModelError::EntityAlreadyExists)` | A model insert collided with an existing row (`with-db`, `mod.rs:266-270`). | 409 | `{"error":"conflict","description":"Resource already exists"}` |
| `Model(ModelError::TenantMismatch)` | `TenantActiveModelExt::set_tenant` was asked to overwrite a different tenant key. | 400 | `{"error":"tenant_mismatch","description":"Model belongs to a different tenant"}` |
| `Model(ModelError::Validation(..))` | A model-level validation failure (`with-db`, `mod.rs:271-272`); same body as the top-level `Validation`. | 400 | `{"errors": <serde_json::Value of the field errors>}` |
| **every remaining variant**, each listed by name in the match | The internal/infrastructure variants (`DB`, `IO`, `Redis`, `Sqlx`, `Tera`, `YAML`, `Message`, `Any`, `InternalServerError`, the remaining `Model` variants — `DbErr`, `Any`, `Message`, `Jwt` — etc.; see the full list below) | **500** | `{"error":"internal_server_error","description":"Internal Server Error"}` |

Every response, regardless of variant, is first logged at `tracing::error!` with `error.msg` / `error.details` fields before the match runs (`src/controller/mod.rs:184-202`).

## `ErrorDetail` — the response body shape

```rust
#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
}
```
(`src/controller/mod.rs:133-142`)

Constructors:

| Fn | Signature | Behavior |
|---|---|---|
| `ErrorDetail::new` | `new<T1, T2>(error: T1, description: T2) -> Self` (`:147`) | Sets `error`; sets `description` to `None` if the passed description is an empty string, `Some(..)` otherwise. `errors` is always `None`. |
| `ErrorDetail::with_reason` | `with_reason<T>(error: T) -> Self` (`:161`) | Sets only `error`; `description`/`errors` are `None`. |

The body is always wrapped in the crate's own `Json<T>` type (`src/controller/mod.rs:170-178`, a thin `axum::Json` newtype), not raw `axum::Json`.

## Constructor helpers on `Error`

`src/errors.rs:153-177`:

| Fn | Signature | Notes |
|---|---|---|
| `Error::wrap` | `wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self` (`:154`) | Boxes any error into `Error::Any(Box::new(err))`. Does **not** call `.bt()` (backtrace capture is commented out). |
| `Error::msg` | `msg(err: impl std::error::Error + Send + Sync + 'static) -> Self` (`:158`) | Stringifies the error's `Display` into `Error::Message(err.to_string())`. Also does not capture a backtrace. |
| `Error::string` | `string(s: &str) -> Self` (`:162`) | Builds `Error::Message(s.to_string())` directly from a string slice. `#[must_use]`. |
| `Error::bt` | `bt(self) -> Self` (`:166`) | Captures `std::backtrace::Backtrace::capture()`. If the backtrace status is `Disabled` or `Unsupported` (i.e. `RUST_BACKTRACE` is unset), returns `self` unchanged — no allocation, no wrapping. Otherwise wraps `self` in `Error::WithBacktrace`. `#[must_use]`. |

Both `wrap`/`msg` are cheap-conversion helpers for turning a foreign `std::error::Error` into the crate's `Error` at a call site (e.g. inside a handler using `.map_err(Error::wrap)`); `bt` is the opt-in backtrace wrapper used internally (e.g. the hand-written `From<serde_json::Error>` impl at `src/errors.rs:24-28` does `Self::JSON(val).bt()`).

## Controller helper functions

Free functions in `src/controller/mod.rs` for the common HTTP-facing variants, each returning `Result<U>` (i.e. always `Err(..)`):

| Fn | Signature | file:line |
|---|---|---|
| `unauthorized` | `unauthorized<T: Into<String>, U>(msg: T) -> Result<U>` | `:112` |
| `bad_request` | `bad_request<T: Into<String>, U>(msg: T) -> Result<U>` | `:121` |
| `not_found` | `not_found<T>() -> Result<T>` | `:130` |

All three are re-exported from `loco_rs::prelude`.

## Full variant list

The complete `#[non_exhaustive] enum Error` (`src/errors.rs:32-151`), with feature gates where present:

| Variant | Feature gate |
|---|---|
| `WithBacktrace { inner: Box<Self>, backtrace: Box<Backtrace> }` | — |
| `Message(String)` | — |
| `QueueProviderMissing` | — |
| `TaskNotFound(String)` | — |
| `Scheduler(#[from] crate::scheduler::Error)` | — |
| `Axum(#[from] axum::http::Error)` | — |
| `Tera(#[from] tera::Error)` | — |
| `JSON(serde_json::Error)` | — (hand-rolled `From`, not `#[from]`, so it can call `.bt()`) |
| `JsonRejection(#[from] JsonRejection)` | — |
| `YAMLFile(#[source] serde_yaml::Error, String)` | — |
| `YAML(#[from] serde_yaml::Error)` | — |
| `EmailSender(#[from] lettre::error::Error)` | — |
| `Smtp(#[from] smtp::Error)` | — |
| `Worker(String)` | — |
| `IO(#[from] std::io::Error)` | — |
| `DB(#[from] sea_orm::DbErr)` | `with-db` |
| `ParseAddress(#[from] AddressError)` | — |
| `Unauthorized(String)` | — |
| `NotFound` | — |
| `BadRequest(String)` | — |
| `CustomError(StatusCode, ErrorDetail)` | — |
| `InternalServerError` | — |
| `InvalidHeaderValue(#[from] InvalidHeaderValue)` | — |
| `InvalidHeaderName(#[from] InvalidHeaderName)` | — |
| `InvalidMethod(#[from] InvalidMethod)` | — |
| `Model(#[from] crate::model::ModelError)` | `with-db` |
| `Redis(#[from] redis::RedisError)` | `worker_redis` |
| `Sqlx(#[from] sqlx::Error)` | `worker` |
| `Storage(#[from] crate::storage::StorageError)` | — |
| `Cache(#[from] crate::cache::CacheError)` | — |
| `Generators(#[from] loco_gen::Error)` | `debug_assertions` |
| `VersionCheck(#[from] depcheck::VersionCheckError)` | — |
| `Any(#[from] Box<dyn std::error::Error + Send + Sync>)` | — |
| `Validation(#[from] ModelValidationErrors)` | — |
| `AxumFormRejection(#[from] axum::extract::rejection::FormRejection)` | — |

## Removed variants (breaking as of the 1.0 error-enum narrowing)

Commit `4a4a84ee` ("narrow the Error enum — drop 4 low-value/leaky variants") removed four variants that are **confirmed absent** from current source (`src/errors.rs`):

- `EnvVar(#[from] std::env::VarError)`
- `Hash(String)`
- `SemVer(#[from] semver::Error)`
- `TaskJoinError(#[from] tokio::task::JoinError)`

Any code that constructs or `match`es these four no longer compiles. Combined with `#[non_exhaustive]`, every downstream `match Error { .. }` must carry a `_ =>` arm — this was already required before the removal, but the removal is a reminder that new/removed variants must not break exhaustive matches, and user code must not attempt to rely on exhaustiveness.
