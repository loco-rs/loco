+++
title = "Add middleware"
description = "Enable a built-in middleware through YAML config, and write a custom MiddlewareLayer when the built-ins don't cover what you need."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 15
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/extras/pluggability/"]

[extra]
lead = ""
toc = true
top = false
+++

**Goal:** turn on one of Loco's 13 built-in middlewares, or write your own when none of them fit, and confirm it's actually running.

This assumes a working app. For the full config-key/knob table for every built-in middleware, see the [Middleware catalog reference](@/docs/reference/middleware.md).

## 1. Enable a built-in middleware via config

Every middleware lives under `server.middlewares.<key>` in your environment YAML (`config/development.yaml`, `config/production.yaml`, ...). Most are disabled by default; a few (`catch_panic`, `etag`, `logger`, `request_id`, and `fallback` outside `Production`) are enabled unless you write the key at all.

Enable `remote_ip` (useful behind a proxy/load balancer) and `compression`:

```yaml
server:
  middlewares:
    remote_ip:
      enable: true
    compression:
      enable: true
```

> **Watch out:** for middlewares that are enabled *by default* (e.g. `etag`, `catch_panic`), writing the key at all — even as `{}` — replaces the framework's own default with the struct's own `#[serde(default)]`, which resolves `enable` to `false` unless you set `enable: true` explicitly. Don't add a middleware's key to config unless you also intend to set `enable`.

## 2. Verify it's registered

```sh
cargo loco middleware --config
```

```sh
limit_payload          {"body_limit":{"Limit":2000000}}
cors                   (disabled)
catch_panic            {"enable":true}
etag                   {"enable":true}
remote_ip              {"enable":true,"source":"RightmostXForwardedFor"}
compression            {"enable":true}
timeout_request        (disabled)
static                 (disabled)
secure_headers         (disabled)
logger                 {"config":{"enable":true},"environment":"development"}
request_id             {"enable":true}
fallback               {"enable":true,"code":200,"file":null,"not_found":null}
powered_by             {"ident":"loco.rs"}
```

`cargo loco middleware` (without `--config`) prints just the enabled/disabled state.

## 3. Common examples

Set a request body size limit:

```yaml
server:
  middlewares:
    limit_payload:
      body_limit: 5mb   # or "disable" to remove the limit entirely
```

Turn on CORS (disabled by default — note the field is `expose_headers`, **plural**):

```yaml
server:
  middlewares:
    cors:
      enable: true
      allow_origins:
        - https://example.com
      allow_headers:
        - Content-Type
      allow_methods:
        - GET
        - POST
      expose_headers:
        - X-Custom-Header
      max_age: 3600
```

Serve static assets or an SPA — see [Serve static & SPA assets](@/docs/how-to/serve-assets.md) for the full walkthrough:

```yaml
server:
  middlewares:
    static:
      enable: true
      folder:
        uri: "/static"
        path: "assets/static"
```

Then use the extractor for a middleware that exposes one, e.g. `RemoteIP`:

```rust
use loco_rs::prelude::*;

#[debug_handler]
pub async fn list(ip: RemoteIP, State(ctx): State<AppContext>) -> Result<Response> {
    tracing::info!(%ip, "handling request");
    format::json(Entity::find().all(&ctx.db).await?)
}
```

## 4. Apply a middleware to a single route instead of globally

Config-driven middleware always applies to every route in the app. To scope a `tower::Layer` to one controller or route, use `Routes::layer` — see [Add a controller § 7](@/docs/how-to/add-controller.md#7-apply-a-tower-layer-to-just-one-controller-or-route).

## 5. Write a custom middleware

Implement the `MiddlewareLayer` trait:

```rust
pub trait MiddlewareLayer {
    fn name(&self) -> &'static str;
    fn is_enabled(&self) -> bool { true } // default
    fn config(&self) -> serde_json::Result<serde_json::Value>;
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>>;
}
```

A minimal example that stamps every response with a custom header, config-toggleable like the built-ins:

```rust
// src/middlewares/hello.rs
use axum::{http::HeaderValue, response::Response, Router as AXRouter};
use loco_rs::{app::AppContext, controller::middleware::MiddlewareLayer, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HelloHeader {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for HelloHeader {
    fn name(&self) -> &'static str {
        "hello_header"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(axum::middleware::map_response(add_header)))
    }
}

async fn add_header(mut res: Response) -> Response {
    res.headers_mut()
        .insert("X-Hello", HeaderValue::from_static("loco"));
    res
}
```

Register it alongside (or instead of) the default stack by overriding the `middlewares` hook on `App` in `src/app.rs`:

```rust
impl Hooks for App {
    // ...
    fn middlewares(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
        let mut mids = middleware::default_middleware_stack(ctx);
        mids.push(Box::new(middlewares::hello::HelloHeader { enable: true }));
        mids
    }
}
```

Remember the ordering rule: `AppRoutes::to_router` applies this `Vec` one `.layer(...)` call at a time, and each new layer wraps the router as the **outer** layer — so the middleware **last** in the vec is the **first** to see an incoming request (LIFO). Push your custom middleware onto whichever end of the vec matches where it needs to sit relative to `logger`/`catch_panic`/etc. See the [Middleware catalog § stack ordering](@/docs/reference/middleware.md#stack-ordering-build-order-vs-request-order-lifo) for the full explanation and the built-in coding order.

## Verify

```sh
cargo loco middleware --config   # confirm your custom entry and its config appear
curl -i localhost:5150/          # confirm the custom header/behavior shows up
```

## Next

- [Middleware catalog reference](@/docs/reference/middleware.md) — every built-in middleware's config key and knobs
- [Serve static & SPA assets](@/docs/how-to/serve-assets.md)
- [Handle errors](@/docs/how-to/handle-errors.md)
