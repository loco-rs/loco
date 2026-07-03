+++
title = "Handle errors"
description = "Return unauthorized/bad_request/not_found from a handler, build a CustomError with an arbitrary status, and know what status code the framework sends for everything else."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 14
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

**Goal:** return the right HTTP status and JSON error body from a handler, without hand-writing `impl IntoResponse` yourself.

This assumes a working controller — see [Add a controller](@/docs/how-to/add-controller.md). Every handler that returns `loco_rs::Result<T>` (i.e. `Result<T, Error>`) gets its error automatically converted to an HTTP response by `impl IntoResponse for Error` — you never call `.into_response()` on an error yourself. For the exhaustive variant list and constructors, see the [Error model reference](@/docs/reference/errors.md).

## 1. The three common-case helpers

`loco_rs::prelude` re-exports three free functions for the HTTP-facing error variants you'll reach for most:

```rust
use loco_rs::prelude::*;

async fn get_one(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let Some(item) = find_item(&ctx, id).await? else {
        return not_found();
    };
    format::json(item)
}

async fn login(State(ctx): State<AppContext>, Json(params): Json<LoginParams>) -> Result<Response> {
    let Ok(user) = find_user(&ctx, &params.email).await else {
        return unauthorized("invalid credentials");
    };
    // ...
    format::json(user)
}

async fn create(Json(params): Json<CreateParams>) -> Result<Response> {
    if params.title.is_empty() {
        return bad_request("title is required");
    }
    // ...
    format::empty()
}
```

Each returns `Result<U>` (always the `Err` arm), so `return unauthorized(msg)` type-checks against any handler's `Result<Response>` return type. All three are also just regular ways to construct an `Error` and propagate it with `?` from a helper function you call from the handler.

| Fn | HTTP status | Response body | Notes |
|---|---|---|---|
| `not_found()` | 404 | `{"error":"not_found","description":"Resource was not found"}` | Takes no message. |
| `unauthorized(msg)` | 401 | `{"error":"unauthorized","description":"You do not have permission to access this resource"}` | `msg` is logged (`tracing::warn!`) but **not** sent to the client. |
| `bad_request(msg)` | 400 | `{"error":"Bad Request","description":"<msg>"}` | `msg` **is** sent to the client. |

## 2. Everything else falls through to 500

`Error` is `#[non_exhaustive]` with ~28 more variants (`DB`, `Model`, `IO`, `Tera`, `Message`, `InternalServerError`, ...). Only seven variants get a specific status; the framework's `IntoResponse` match ends in a wildcard arm — **every other variant becomes `500 Internal Server Error`** with body `{"error":"internal_server_error","description":"Internal Server Error"}`.

In practice this means: if you `?`-propagate a `sea_orm::DbErr`, an `std::io::Error`, or anything else that converts into `Error` via `#[from]`, and you haven't matched it explicitly, the client gets a generic 500 — which is usually what you want (don't leak internals), and every response is logged at `tracing::error!` first regardless of variant, so you still see the real cause server-side.

The full variant → status table, including `Validation` (→ 400, from the `validator` crate) and `JsonRejection` (→ axum's own rejection status), is in the [Error model reference](@/docs/reference/errors.md).

## 3. Return an arbitrary status: `Error::CustomError`

When none of the built-in helpers fit — a `409 Conflict`, a `429 Too Many Requests`, a body shape the framework doesn't produce — build one directly:

```rust
use loco_rs::prelude::*;
use loco_rs::controller::ErrorDetail;
use axum::http::StatusCode;

async fn create(Json(params): Json<CreateParams>) -> Result<Response> {
    if already_exists(&params).await? {
        return Err(Error::CustomError(
            StatusCode::CONFLICT,
            ErrorDetail::new("conflict", "a resource with this name already exists"),
        ));
    }
    format::empty()
}
```

`Error::CustomError(StatusCode, ErrorDetail)` passes both the status and the body through **unchanged** — it's the one variant the response mapping doesn't rewrite. `ErrorDetail::new(error, description)` sets both fields (an empty description collapses to `None`); `ErrorDetail::with_reason(error)` sets only `error`. The response body is always `{error, description, errors}` with `None` fields omitted from the JSON.

## 4. Convert a foreign error without a dedicated variant

Use `Error::wrap`/`Error::msg` at a `?`/`.map_err(..)` call site to fold any `std::error::Error` into `Error::Any`/`Error::Message` (both fall through to the 500 catch-all above):

```rust
let parsed: MyType = serde_json::from_str(&raw).map_err(Error::wrap)?;
```

Reach for `Error::string("...")` when you have a plain `&str`/message and no source error to wrap.

## 5. Content-type-aware error handling

If an endpoint needs to render errors differently for HTML vs. JSON clients, match on both the fallible call's `Result` and the negotiated format in one place — see [Respond with different formats](@/docs/how-to/respond-formats.md#4-combine-format-negotiation-with-error-handling).

## Verify

```sh
curl -i localhost:5150/notes/999999   # -> 404 {"error":"not_found",...}
curl -i -X POST localhost:5150/auth/login -d '{"email":"x","password":"y"}' -H 'content-type: application/json'
# -> 401 {"error":"unauthorized",...}
```

## Next

- [Validate requests](@/docs/how-to/validate-requests.md) — the `Validation` variant and structured field errors
- [Respond with different formats](@/docs/how-to/respond-formats.md)
- [Error model reference](@/docs/reference/errors.md) — full variant list and constructors
