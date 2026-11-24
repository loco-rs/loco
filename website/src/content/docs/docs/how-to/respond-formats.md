---
title: Respond with different formats
description: "Use the format:: response helpers (json, text, html, yaml, redirect, empty) and negotiate content type with RespondTo/Format."
sidebar:
  order: 13
---

**Goal:** return the right response shape (JSON, HTML, plain text, YAML, a redirect, an empty body) from a handler, and — when a single endpoint must serve more than one format — pick the shape based on the request's `Content-Type`/`Accept` header.

This assumes a working controller — see [Add a controller](/docs/how-to/add-controller). All helpers live in the `format` module (`loco_rs::controller::format`, re-exported as `format` from the prelude). Keep handlers returning `Result<impl IntoResponse>` (or `Result<Response>`) so you can freely swap which `format::*` call you return.

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

`format::view`/`format::template` render HTML from a Tera view or an inline template string — see [Render server-side views](/docs/how-to/render-views).

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

See [Handle errors](/docs/how-to/handle-errors) for what "the framework's default error rendering" produces for each `Error` variant.

## 5. One action, HTML or JSON: `format::respond_to`

`format::respond_to` is the Rails `respond_to do |format|` block. Give it a branch per format; only the chosen branch runs. A request with no `Accept`/`Content-Type` (or an unrecognised one) gets the first branch, and a request for a format with no branch gets `406 {"error":"not_acceptable"}`:

```rust
use loco_rs::prelude::*;

pub async fn show(
    Format(respond_to): Format,
    ViewEngine(v): ViewEngine<TeraView>,
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let post = Model::find_by_id(&ctx.db, id).await?;
    format::respond_to(&respond_to)
        .html(|| format::render().view(&v, "posts/show.html", data!({"post": &post})))
        .json(|| format::json(&post))
        .render()
}
```

`.xml(..)` and `.any(..)` are also available; `.any` answers every format no earlier branch took.

When the HTML view and the JSON body show the same data, `format::respond_with` is the one-liner — the view gets `data` under `key`, JSON gets `data` itself:

```rust
format::respond_with(&v, &respond_to, "posts/show.html", PostResponse::from(&post))
```

## 6. Serve every route as JSON: the `respond_to` middleware

Rails apps answer `/posts/1.json` without extra routes. Turn on the `respond_to` middleware for the same thing, plus an optional prefix that mounts every route again as JSON:

```yaml
server:
  middlewares:
    respond_to:
      enable: true
      extension: true          # /posts/1.json (default true)
      prefix: /api/v1          # /api/v1/posts/1 (optional)
      sign_in_path: /session/new  # redirects here become 401 (optional)
```

The middleware strips `.json` or the prefix, routes the request to the same handler with `Accept: application/json`, and the `Format`/`RespondTo` extractor reports `Json`. Responses built for a browser are turned into JSON:

| The handler answers | The client gets |
|---|---|
| a redirect | `200 {"ok":true,"redirect":"/posts"}` |
| a redirect to `sign_in_path` | `401 {"error":"signed_out","redirect":"/session/new"}` |
| an HTML page (no JSON branch) | `406 {"error":"not_acceptable","description":"This action has no JSON response"}` |
| an HTML error page | `{"error":"not_found"}` (or `forbidden`, `unauthorized`, `bad_request`, `error`) with the same status |
| JSON | unchanged |

Cookies and other headers are kept. A real static file ending in `.json` is still served: when the stripped path is not found, the original path is tried. Plain `Accept: application/json` requests to the normal path are left alone — only `.json` and the prefix opt into the conversion.

## Verify

```sh
curl -s localhost:5150/notes/1 -H 'accept: application/json'
curl -s localhost:5150/notes/1 -H 'accept: text/html'
# with the respond_to middleware enabled
curl -s localhost:5150/notes/1.json
curl -s localhost:5150/api/v1/notes/1
```

## Next

- [Render server-side views](/docs/how-to/render-views)
- [Handle errors](/docs/how-to/handle-errors)
- [Validate requests](/docs/how-to/validate-requests)
