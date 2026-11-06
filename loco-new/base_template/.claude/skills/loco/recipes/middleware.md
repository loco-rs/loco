# Recipe: middleware

Rails puts cross-cutting behaviour in `ApplicationController` via
`before_action`. Rust has no class inheritance, so Loco splits that in two:

- **Per-request infrastructure** (CORS, compression, timeouts, limits, logging)
  is **middleware**, configured in `config/<env>.yaml` — not code.
- **Per-handler requirements** ("must be logged in") are **extractors** in the
  handler signature. See `auth.md`.

Loco ships 13 built-in middlewares, each implementing `MiddlewareLayer` and
configured under `server.middlewares.<key>`.

## Two traps that account for most middleware bugs

### 1. Ordering is LIFO

Middlewares are applied in config order, but Axum's `Router::layer` wraps the
*existing* router as the **outer** layer. So:

> the LAST middleware is the FIRST to meet the outside world

An inbound request traverses the stack in **reverse** of the listed order —
`powered_by` first, `limit_payload` last (immediately before your handler) — and
the response flows back out the other way. If you are reasoning about "which
middleware sees the request first," invert the list.

### 2. Writing the key at all flips the default

Every middleware is `Option<T>`. When the key is **absent**, the framework's own
default applies — and `catch_panic`, `etag`, `logger`, `request_id`, and (outside
production) `fallback` default to **enabled**.

The moment you write the key — even as an empty mapping `etag: {}` — the
struct's `#[serde(default)]` takes over, and that resolves `enable` to `false`.

```yaml
# Silently DISABLES etag:
etag: {}

# Correct:
etag:
  enable: true
```

**Do not write a middleware's key unless you also set `enable` explicitly.**

## Configuring

```yaml
server:
  middlewares:
    limit_payload:
      enable: true
      body_limit: 5mb          # or "disable"
    cors:
      enable: true
      allow_origins: ["https://example.com"]
      allow_headers: ["*"]
      allow_methods: ["GET", "POST"]
      allow_credentials: false
      max_age: 3600
    timeout_request:
      enable: true
      timeout: 5000            # milliseconds
    compression:
      enable: true
    secure_headers:
      enable: true
      preset: github           # github | owasp | empty
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/static"
        path: "assets/static"
      fallback: "assets/static/404.html"
    remote_ip:
      enable: true
      source: RightmostXForwardedFor
```

The full catalog — every key, its type, and its real default — is in the docs
under **Reference → Middleware catalog**. Check there before inventing a key;
an unknown key is silently ignored, so a typo produces no error and no effect.

## Scoping a layer to one route group

`Routes` takes a tower layer directly, which applies to that group only:

```rust
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/admin")
        .layer(SomeLayer::new())
        .add("/", get(index))
}
```

## Writing your own

Implement `MiddlewareLayer`:

```rust
pub trait MiddlewareLayer {
    fn name(&self) -> &'static str;
    fn is_enabled(&self) -> bool { true }
    fn config(&self) -> serde_json::Result<serde_json::Value>;
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>>;
}
```

Before you do: check the catalog. Nine times out of ten the behaviour you want
is a built-in you have not found yet, and a custom layer that duplicates one is
a P1 violation.

## Verifying

```sh
cargo loco routes      # shows registered routes
cargo loco doctor      # config sanity
```
