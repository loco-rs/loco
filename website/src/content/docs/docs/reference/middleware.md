---
title: Middleware catalog
description: "Every built-in middleware: config key, struct, default state, and knobs."
sidebar:
  order: 5
---

Loco ships 13 built-in middlewares, all implementing the `MiddlewareLayer`
trait (`src/controller/middleware/mod.rs:46-72`). Each is configured under
`server.middlewares.<key>` in your environment YAML (`src/config/server.rs:44`)
and is optional (`Option<T>`) — omit the key entirely to get the framework's
own default; supply the key (even as `{}`) to take over its `serde` defaults
instead (see the callout below).

## The `MiddlewareLayer` trait

```rust
pub trait MiddlewareLayer {
    fn name(&self) -> &'static str;
    fn is_enabled(&self) -> bool { true } // default
    fn config(&self) -> serde_json::Result<serde_json::Value>;
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>>;
}
```

`src/controller/middleware/mod.rs:46-72`.

> **Config-key-present flips the default.** For every middleware below whose
> "default enabled" is `true` (`catch_panic`, `etag`, `logger`, `request_id`,
> and — outside Production — `fallback`), that default comes from
> `default_middleware_stack`'s own fallback value, used only when the key is
> **absent** from config (`Option` is `None`,
> `src/controller/middleware/mod.rs:76-171`). If you write the key at all —
> even as an empty mapping (`etag: {}`) — the struct's own `#[serde(default)]`
> takes over, which resolves `enable` to `false` unless you set
> `enable: true` explicitly. In short: don't write a middleware's key in
> config unless you intend to also set `enable`.

## Stack ordering: build order vs. request order (LIFO)

`default_middleware_stack(ctx)` (`mod.rs:76-171`) returns middlewares as a
`Vec` in the coding order below (limit_payload → … → powered_by).
`AppRoutes::to_router` applies them in that same order, one `app.layer(...)`
call at a time (`src/controller/app_routes.rs:305-309`). Axum's
`Router::layer` wraps the *existing* router with each new layer as the
**outer** layer, so:

> "the LAST middleware is the FIRST to meet the outside world (a user request
> starting), or 'LIFO' order" — `src/controller/app_routes.rs:283-286`.

So an inbound request actually passes through the stack in the **reverse**
of the table order — `powered_by` first, `limit_payload` last (right before
the route handler) — and the response flows back out the opposite way.

## Full middleware set

Table order = coding/config order (`default_middleware_stack`, `mod.rs:80-169`).

### 1. `limit_payload`

- **Config key:** `limit_payload` · **Struct:** `limit_payload::LimitPayload` (`limit_payload.rs:26`)
- **Default:** effectively always enabled — `is_enabled()` is hard-coded `true` (`limit_payload.rs:70-72`); there is no `enable` field. To turn it off, set `body_limit: disable`.
- **Purpose:** caps the request body size via Axum's `DefaultBodyLimit`.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `body_limit` | `DefaultBodyLimitKind` (`"<size>"` e.g. `"5mb"`, or `"disable"`) | `2mb` (2,000,000 bytes) — `limit_payload.rs:43-45` |

### 2. `cors`

- **Config key:** `cors` · **Struct:** `cors::Cors` (`cors.rs:19-42`)
- **Default:** **disabled**.
- **Purpose:** Cross-Origin Resource Sharing headers.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `false` |
| `allow_origins` | `Vec<String>` | `["*"]` |
| `allow_headers` | `Vec<String>` | `["*"]` |
| `allow_methods` | `Vec<String>` | `["*"]` |
| `expose_headers` | `Vec<String>` | `[]` (empty) |
| `allow_credentials` | `bool` | `false` |
| `max_age` | `Option<u64>` (seconds) | `None` |
| `vary` | `Vec<String>` | `["origin", "access-control-request-method", "access-control-request-headers"]` |

> The field is `expose_headers` (plural) — `cors.rs:32`.

### 3. `catch_panic`

- **Config key:** `catch_panic` · **Struct:** `catch_panic::CatchPanic { enable }` (`catch_panic.rs:18-22`)
- **Default:** **enabled**.
- **Purpose:** catches panics in request handlers, logs them, and returns `500 Internal Server Error` instead of dropping the connection.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `true` (framework default when key absent) |

### 4. `etag`

- **Config key:** `etag` · **Struct:** `etag::Etag { enable }` (`etag.rs:27-31`)
- **Default:** **enabled**.
- **Purpose:** compares `If-None-Match` against the response `ETag` and returns `304 Not Modified` on a match.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `true` (framework default when key absent) |

### 5. `remote_ip`

- **Config key:** `remote_ip` · **Struct:** `remote_ip::RemoteIpMiddleware { enable, source }` (`remote_ip.rs`)
- **Default:** **disabled**.
- **Purpose:** resolves the client IP from a single, trusted source (a proxy header, or the raw socket address). Implemented as a thin wrapper over the [`axum-client-ip`](https://docs.rs/axum-client-ip) crate.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `false` |
| `source` | `axum_client_ip::ClientIpSource` | `RightmostXForwardedFor` |

`source` selects exactly one trusted source — there is no proxy-chain walking and no CIDR trust list. Valid values (serialized as the bare variant name, e.g. `source: XRealIp`): `RightmostXForwardedFor` (last value of the last `X-Forwarded-For` header, taken verbatim), `RightmostForwarded` (RFC 7239 `Forwarded` header), `CfConnectingIp` (Cloudflare), `CloudFrontViewerAddress` (AWS CloudFront), `FlyClientIp` (Fly.io), `TrueClientIp` (Akamai/Cloudflare), `XEnvoyExternalAddress` (Envoy/Istio), `XRealIp` (nginx), or `ConnectInfo` (the raw socket peer address, no header involved).

> **BREAKING (was `trusted_proxies: Option<Vec<String>>`):** the old middleware hand-rolled `X-Forwarded-For` parsing, walking the header right-to-left and skipping any IP in a configurable trusted-proxy CIDR list (or a built-in RFC-1918 + loopback list) — i.e. it could see through a chain of one or more trusted proxies. The new `source` field trusts exactly **one** hop and applies no CIDR filtering at all. If you run multiple hops (CDN → load balancer → ingress), configure your innermost hop to compute and set the correct client IP itself, and point `source` at whatever header it writes (or pick a provider-specific source like `CfConnectingIp`).

### 6. `compression`

- **Config key:** `compression` · **Struct:** `compression::Compression { enable }` (`compression.rs:14-18`)
- **Default:** **disabled**.
- **Purpose:** compresses response bodies (`tower_http::compression::CompressionLayer`).
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `false` |

### 7. `timeout_request`

- **Config key:** `timeout_request` · **Struct:** `timeout::TimeOut { enable, timeout }` (`timeout.rs:23-30`)
- **Default:** **disabled**.
- **Purpose:** aborts a request and returns `408 Request Timeout` if it runs longer than `timeout`.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `false` |
| `timeout` | `u64` (milliseconds) | `5000` (`timeout.rs:38-40`) |

### 8. `static`

- **Config key:** `static` (Rust field `static_assets`, `#[serde(rename = "static")]`, `mod.rs:197-199`) · **Struct:** `static_assets::StaticAssets` (`static_assets.rs:24-43`)
- **Default:** **disabled**.
- **Purpose:** serves a static-file folder, with an optional fallback file for SPA routing.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `false` |
| `must_exist` | `bool` | `true` |
| `folder.uri` | `String` | `"/static"` |
| `folder.path` | `PathBuf` | `"assets/static"` |
| `fallback` | `PathBuf` | `"assets/static/404.html"` |
| `precompressed` | `bool` | `false` (serves `.gz` variants when `true`) |
| `cache_control` | `Option<String>` | `None` (e.g. `"max-age=31536000"`) |

> Under the `embedded_assets` feature, this swaps at compile time for `static_assets_embedded::StaticAssets` (`mod.rs:21-27`) — same config key (`"static"`) and knob surface, assets baked into the binary instead of read from disk.

### 9. `secure_headers`

- **Config key:** `secure_headers` · **Struct:** `secure_headers::SecureHeader { enable, preset, overrides }` (`secure_headers.rs:78-86`)
- **Default:** **disabled**.
- **Purpose:** injects a preset bundle of security headers (CSP, X-Frame-Options, etc.), individually overridable.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `false` |
| `preset` | `String` | `"github"` (`secure_headers.rs:94-96`) — other presets: `owasp`, `empty` (`secure_headers.json`) |
| `overrides` | `Option<BTreeMap<String, String>>` | `None` |

### 10. `logger`

- **Config key:** `logger` · **Struct:** `logger::Config { enable }` → `logger::Middleware` via `logger::new(config, &env)` (`logger.rs:21-25, 36-42`)
- **Default:** **enabled**.
- **Purpose:** `TraceLayer`-based request logging (method, URI, version, user agent, request ID, environment).
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `true` (framework default when key absent) |

### 11. `request_id`

- **Config key:** `request_id` · **Struct:** `request_id::RequestId { enable }` (`request_id.rs:28-32`)
- **Default:** **enabled**.
- **Purpose:** ensures every request has an `x-request-id` header (sanitizes an incoming one or generates a UUID v4), and exposes it to handlers as `LocoRequestId(String)` via `.get()` (`request_id.rs:63-72`).
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `true` (framework default when key absent) |

### 12. `fallback`

- **Config key:** `fallback` · **Struct:** `fallback::Fallback { enable, code, file, not_found }` (`fallback.rs:17-37`); `StatusCodeWrapper(pub StatusCode)` (`fallback.rs:15`)
- **Default:** enabled **only when `environment != Production`** (`mod.rs:158-167`).
- **Purpose:** serves a response for unmatched routes — a file, a plain message, or the bundled `fallback.html` — instead of Axum's bare 404.
- **Knobs:**

| Name | Type | Default |
|---|---|---|
| `enable` | `bool` | `true` outside Production, `false` in Production (framework default when key absent) |
| `code` | `StatusCode` (as `u16`) | `404` (`NOT_FOUND`) — `default_status_code`, `fallback.rs:39-41`. Set it only for the unusual case where an unmatched route should answer with something else |
| `file` | `Option<String>` | `None` — path to a file served as the fallback body |
| `not_found` | `Option<String>` | `None` — a plain-text message served as the fallback body |

If neither `file` nor `not_found` is set, the bundled `fallback.html` is served.

### 13. `powered_by`

- **Config key:** none — **not** part of `middleware::Config`; controlled by `server.ident: Option<String>` (`src/config/server.rs:40`). Struct: `powered_by::Middleware` via `powered_by::new(ctx.config.server.ident.as_deref())` (`powered_by.rs:27-58`)
- **Default:** **enabled**, sets `Server`-identifying header `X-Powered-By: loco.rs`.
- **Purpose:** sets an `X-Powered-By` response header.
- **Knobs (via `server.ident`, not `enable`):**

| `server.ident` value | Effect |
|---|---|
| absent / `None` | `X-Powered-By: loco.rs` (default) |
| `""` (empty string) | middleware disabled — no header |
| any other string | `X-Powered-By: <string>` |

## Editing the stack: `MiddlewareStackExt`

`Hooks::middlewares` hands you the `Vec<Box<dyn MiddlewareLayer>>` that
`default_middleware_stack` produced. To tweak it rather than rebuild it from
scratch, the `MiddlewareStackExt` trait
(`src/controller/middleware/mod.rs:187-198`) adds four Rails-style edits,
implemented for that `Vec` and re-exported from `loco_rs::prelude`
(`src/prelude.rs:39`), so it is already in scope in generated apps:

| Method | Effect |
|---|---|
| `insert_before(name, middleware)` | Insert immediately before the first middleware whose `name()` matches |
| `insert_after(name, middleware)` | Insert immediately after it |
| `replace(name, middleware)` | Swap it out in place |
| `delete(name)` | Remove it |

Each returns `&mut Self`, so calls chain. Middlewares are matched by
`MiddlewareLayer::name()`, and positions are positions in the `Vec` — which,
per the LIFO note above, means "before" in `Vec` order is *later* in request
order.

If nothing matches `name`, the operation logs a warning rather than failing:
`insert_before` / `insert_after` still append the new middleware (so it isn't
silently dropped), while `replace` / `delete` leave the stack untouched.

```rust
fn middlewares(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
    let mut stack = loco_rs::controller::middleware::default_middleware_stack(ctx);
    stack.delete("powered_by").insert_after("etag", Box::new(MyMiddleware));
    stack
}
```

## Introspecting the stack

```
cargo loco middleware           # list every middleware and its enabled state
cargo loco middleware --config  # also print each middleware's JSON config
```

`src/cli.rs:100-104`, backed by `list_middlewares` (`src/boot.rs:588-597`), which calls each middleware's `name()`, `is_enabled()`, and `config()`.
