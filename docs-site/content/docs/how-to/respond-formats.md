+++
title = "Respond with different formats"
description = "Use the format:: response helpers (json, text, html, yaml, redirect, empty) and negotiate content type with RespondTo/Format."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 13
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

**Goal:** return the right response shape (JSON, HTML, plain text, YAML, a redirect, an empty body) from a handler, and — when a single endpoint must serve more than one format — pick the shape based on the request's `Content-Type`/`Accept` header.

This assumes a working controller — see [Add a controller](@/docs/how-to/add-controller.md). All helpers live in the `format` module (`loco_rs::controller::format`, re-exported as `format` from the prelude). Keep handlers returning `Result<impl IntoResponse>` (or `Result<Response>`) so you can freely swap which `format::*` call you return.

## 1. Simple responses

```rust
use loco_rs::prelude::*;

async fn as_json() -> Result<Response> {
    format::json(serde_json::json!({ "hello": "world" }))
}

async fn as_text() -> Result<Response> {
    format::text("hello, world")
}

async fn as_html() -> Result<Response> {
    format::html("<h1>hello</h1>")
}

async fn as_yaml() -> Result<Response> {
    format::yaml("openapi: 3.1.0\ninfo:\n  title: my api\n")
    // sets Content-Type: application/yaml
}

async fn nothing() -> Result<Response> {
    format::empty() // 200, empty body
}

async fn empty_object() -> Result<Response> {
    format::empty_json() // 200, body: {}
}

async fn go_elsewhere() -> Result<Response> {
    format::redirect("/dashboard") // axum::response::Redirect::to(..)
}
```

`format::view`/`format::template` render HTML from a Tera view or an inline template string — see [Render server-side views](@/docs/how-to/render-views.md).

## 2. When you need more than a one-liner: `format::render()`

`format::render()` returns a `RenderBuilder` you chain, terminating with one of `.json(..)`, `.html(..)`, `.text(..)`, `.empty()`, `.view(..)`, `.template(..)`, `.redirect(..)`, or `.redirect_with_header_key(..)`:

```rust
async fn get_one(State(ctx): State<AppContext>) -> Result<Response> {
    format::render()
        .etag("some-etag-value")?
        .header("X-Custom", "1")
        .json(load_item(&ctx).await?)
}
```

Available builder methods:

| Method | Purpose |
|---|---|
| `.status(code)` | set the response status (defaults to `200`) |
| `.header(key, value)` | add a single response header |
| `.etag(value)` | set the `ETag` header (errors on non-visible-ASCII input) |
| `.cookies(&[Cookie, ..])` | add one `Set-Cookie` header per cookie |
| `.response()` | escape hatch: hand back the raw `axum::http::response::Builder` |
| `.redirect(to)` | `303 See Other` with `Location: <to>` |
| `.redirect_with_header_key(key, to)` | same, but with a custom header instead of `Location` — e.g. `HX-Redirect` for HTMX |

```rust
async fn htmx_redirect() -> Result<Response> {
    format::render().redirect_with_header_key("HX-Redirect", "/notes")
}
```

## 3. Content negotiation: respond differently per client

Use the `RespondTo` extractor (or its wrapper `Format(pub RespondTo)`) to detect the request's format from `Content-Type` (checked first) or, failing that, `Accept`:

```rust
use loco_rs::prelude::*;

pub async fn get_one(
    respond_to: RespondTo,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    match respond_to {
        RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
        _ => format::json(item),
    }
}
```

`RespondTo` variants: `None` (neither header present/parseable), `Html`, `Json`, `Xml`, `Other(String)` (any other MIME type, preserved verbatim). Both `RespondTo` and `Format` implement `FromRequestParts`, so you can extract either one directly as a handler parameter — `Format(respond_to)` if you prefer the wrapped form.

## 4. Combine format negotiation with error handling

A common pattern: run your fallible logic first, then match on both the `Result` and the format in one place, so error rendering stays consistent per-format:

```rust
pub async fn get_one(
    respond_to: RespondTo,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let res = load_item(&ctx, id).await;

    match res {
        Ok(item) => match respond_to {
            RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
            _ => format::json(item),
        },
        Err(Error::Model(ModelError::Validation(errors))) => match respond_to {
            RespondTo::Html => format::html(&format!("<html><body>errors: {errors:?}</body></html>")),
            _ => bad_request("opaque message: cannot respond!"),
        },
        // unhandled error kinds: let the framework's default error rendering take over
        Err(err) => Err(err),
    }
}
```

See [Handle errors](@/docs/how-to/handle-errors.md) for what "the framework's default error rendering" produces for each `Error` variant.

## Verify

```sh
curl -s localhost:5150/notes/1 -H 'accept: application/json'
curl -s localhost:5150/notes/1 -H 'accept: text/html'
```

## Next

- [Render server-side views](@/docs/how-to/render-views.md)
- [Handle errors](@/docs/how-to/handle-errors.md)
- [Validate requests](@/docs/how-to/validate-requests.md)
