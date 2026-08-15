---
title: Serve static & SPA assets
description: Serve a static folder (or a single-page app) with the built-in static middleware, then optionally embed everything into the binary with the embedded_assets feature.
sidebar:
  order: 16
---

**Goal:** serve files (images, CSS, JS, a compiled SPA bundle) directly from Loco, either from disk or embedded into the compiled binary.

This assumes a working app. For the full knob table, see the `static` entry in the [Middleware catalog reference](/docs/reference/middleware).

## 1. Put files under `assets/static/`

```
assets/
├── static/
│   ├── image.png
│   └── 404.html
└── views/
```

`assets/` sits at your project root, next to `src/` and `config/`.

## 2. Enable the `static` middleware

```yaml
# config/development.yaml
server:
  middlewares:
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/static"
        path: "assets/static"
      fallback: "assets/static/404.html"
```

| Key | Default | Purpose |
|---|---|---|
| `must_exist` | `true` | if `true`, a missing configured folder is a boot-time error |
| `folder.uri` | `/static` | the URL prefix clients request under |
| `folder.path` | `assets/static` | the on-disk folder served |
| `fallback` | `assets/static/404.html` | file served when a requested path doesn't exist |
| `precompressed` | `false` | serve a `.gz` sibling file instead of compressing on the fly, if one exists |
| `cache_control` | `None` | e.g. `"max-age=31536000, public"`; set `null` to disable caching headers entirely |

Reference the served files from HTML/templates as normal:

```html
<img src="/static/image.png" />
```

## 3. Disable the welcome-screen fallback if it's shadowing your assets

Outside `Production`, Loco enables a separate `fallback` middleware by default (the "Loco welcome screen" for unmatched routes), and it takes precedence over `static`. If your static assets aren't showing up as expected in development, disable it:

```yaml
server:
  middlewares:
    fallback:
      enable: false
```

## 4. Serve a single-page app (SPA)

Point `fallback` (on `static_assets`, not the framework-wide `fallback` middleware above) at your SPA's `index.html` so client-side routes resolve correctly on a hard refresh:

```yaml
server:
  middlewares:
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/"
        path: "assets/static"
      fallback: "assets/static/index.html"
```

Any request that doesn't match a real file under `assets/static/` falls back to `index.html`, letting your client-side router take over.

If the SPA is Loco's own clientside mode — a Vite/React frontend in `frontend/`, with TypeScript types generated from your Rust DTOs — this config is already generated for you (pointing at `frontend/dist`). See [Build a typed React SPA](/docs/how-to/build-a-spa).

## 5. Serve precompressed assets

If your build pipeline already produces `.gz` files next to the originals (e.g. `app.js` and `app.js.gz`), turn on `precompressed` and Loco serves the `.gz` variant directly instead of compressing per-request:

```yaml
server:
  middlewares:
    static:
      enable: true
      precompressed: true
```

## 6. Embed assets into the binary with `embedded_assets`

For single-binary deployment (no separate asset directory to ship or mount), enable the `embedded_assets` Cargo feature:

```toml
[dependencies]
loco-rs = { version = "...", features = ["embedded_assets"] }
```

This is a **compile-time swap**, not a separate API — the same `server.middlewares.static` config key and knobs still apply, and your controllers/views don't change at all:

- the `static` middleware's implementation swaps from reading `assets/static/` off disk to serving files baked into the binary at build time
- the Tera view engine (`TeraView`) likewise swaps to an embedded variant that serves compiled-in templates instead of reading `assets/views/` off disk

At build time you'll see log output confirming what got embedded:

```
warning: loco-rs@x.y.z: Discovered directories for assets:
warning: loco-rs@x.y.z:   - /path/to/app/assets/static
warning: loco-rs@x.y.z:   - /path/to/app/assets/views
warning: loco-rs@x.y.z: Found asset: /path/to/app/assets/static/image.png -> /static/image.png
warning: loco-rs@x.y.z: Found 6 asset files
warning: loco-rs@x.y.z: Generated code for 6 static assets and 7 templates
```

Trade-offs: the binary grows by roughly the size of `assets/`, and any asset change requires a full recompile — there's no "edit and refresh" loop like serving from disk. Toggle the feature on/off per build profile (e.g. embedded for release, filesystem for local dev) if that recompile cost is a problem during active asset iteration.

## Verify

```sh
curl -I localhost:5150/static/image.png    # 200, correct Content-Type
curl -I localhost:5150/static/does-not-exist.png   # falls back per `fallback` config
```

## Next

- [Render server-side views](/docs/how-to/render-views)
- [Add middleware](/docs/how-to/add-middleware)
- [Middleware catalog reference](/docs/reference/middleware)
