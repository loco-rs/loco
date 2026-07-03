+++
title = "Render server-side views"
description = "Render HTML with Loco's Tera-based ViewRenderer: create a template, wire the ViewEngine extractor, and use the format::render() builder."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 12
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

**Goal:** return server-rendered HTML from a controller using Loco's built-in [Tera](http://keats.github.io/tera/)-based view engine.

This assumes a working app with the view engine initializer configured (the SaaS/HTML starters have this out of the box). If you generated an API-only app and are adding HTML for the first time, see step 5.

## 1. Create a template

Templates live under `assets/views/` (the `assets/` folder sits next to `src/` and `config/` at your project root):

```html
<!-- assets/views/home/hello.html -->
<html>
  <body>
    <h1>{{ title }}</h1>
  </body>
</html>
```

## 2. Wrap it in a typed view function

Encapsulate the template call so controllers never touch Tera or template paths directly — this is what lets you swap view engines later without touching controllers.

```rust
// src/views/dashboard.rs
use loco_rs::prelude::*;

pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::render().view(&v, "home/hello.html", data!({"title": "Loco"}))
}
```

Add it to `src/views/mod.rs`:

```rust
pub mod dashboard;
```

## 3. Extract the view engine in your controller

`ViewEngine<E>` is a `FromRequestParts` extractor — `TeraView` is the concrete engine Loco supplies. Both come from the prelude.

```rust
// src/controllers/dashboard.rs
use loco_rs::prelude::*;
use crate::views;

pub async fn render_home(ViewEngine(v): ViewEngine<TeraView>) -> Result<impl IntoResponse> {
    views::dashboard::home(v)
}

pub fn routes() -> Routes {
    Routes::new().prefix("home").add("/", get(render_home))
}
```

Register the controller's routes as usual — see [Add a controller](@/docs/how-to/add-controller.md).

`ViewEngine<E>` requires a `TeraLayer` `Extension` to be installed on the router; it panics with `"TeraLayer missing. Is the TeraLayer installed?"` if it isn't. This is wired up by the `ViewEngineInitializer` in `src/initializers/view_engine.rs` (present by default in HTML/HTMX starters) — see step 5 if you need to add it.

## 4. Two ways to render

**`format::view`** — the simplest form, renders directly to an HTML response:

```rust
pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::view(&v, "home/hello.html", data!({"title": "Loco"}))
}
```

**`format::render()`** — a chainable builder when you need more than a bare 200 HTML body. It terminates with `.view(...)`, `.template(...)`, `.html(...)`, or `.json(...)`, and can add headers, an `ETag`, cookies, or a status code first:

```rust
pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::render()
        .etag("home-v1")?
        .cookies(&[axum_extra::extract::cookie::Cookie::new("last_view", "home")])?
        .view(&v, "home/hello.html", data!({"title": "Loco"}))
}
```

`format::render()` also has a `.response()` escape hatch that returns the underlying `axum::http::response::Builder` if you need something the chain doesn't cover, and `.redirect(to)` / `.redirect_with_header_key(key, to)` for redirects (see [Respond with different formats](@/docs/how-to/respond-formats.md)).

For an inline template string with no file on disk, use `format::template(tmpl, data)` (or the builder's `.template(...)`) instead of `.view(...)`.

## 5. Enabling the view engine on an API-only app

If your app was generated `--api`-only, add the initializer:

```rust
// src/initializers/view_engine.rs
use async_trait::async_trait;
use axum::{Extension, Router as AxumRouter};
use loco_rs::{
    app::{AppContext, Initializer},
    controller::views::{engines, ViewEngine},
    Result,
};

pub struct ViewEngineInitializer;

#[async_trait]
impl Initializer for ViewEngineInitializer {
    fn name(&self) -> String {
        "view-engine".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let tera_engine = engines::TeraView::build()?;
        Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
    }
}
```

Register it in `src/app.rs`:

```rust
async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
    Ok(vec![Box::new(initializers::view_engine::ViewEngineInitializer)])
}
```

`TeraView::build()` loads templates from `assets/views` (`DEFAULT_ASSET_FOLDER = "assets"`). Use `TeraView::build_with_post_process(|tera| { ... })` instead if you need to register custom Tera functions (e.g. an i18n `t(...)` function) — see the demo app's `src/initializers/view_engine.rs` for a working example with `fluent-templates`.

## 6. Serving static assets referenced by your templates

Templates that reference `<img src="/static/...">` need the `static` middleware — see [Serve static & SPA assets](@/docs/how-to/serve-assets.md).

## 7. Use a different template engine entirely

Because controllers only depend on `ViewRenderer` (a one-method trait: `render<S: Serialize>(&self, key: &str, data: S) -> Result<String>`), you can substitute Tera for anything else. Implement `ViewRenderer` for your own type, register it via an `Initializer` the same way as `TeraView` above, and swap the extractor's generic parameter (`ViewEngine<TeraView>` → `ViewEngine<YourEngine>`) — no controller logic changes.

## Verify

```sh
cargo loco routes   # confirm GET /home is registered
curl -s localhost:5150/home
```

## Next

- [Respond with different formats](@/docs/how-to/respond-formats.md) — JSON/HTML/YAML and content negotiation
- [Serve static & SPA assets](@/docs/how-to/serve-assets.md)
- [Handle errors](@/docs/how-to/handle-errors.md)
