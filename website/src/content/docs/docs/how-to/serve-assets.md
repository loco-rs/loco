---
title: Serve static & SPA assets
description: Serve a static folder with the built-in static middleware, a single-page app build with the spa middleware, then optionally embed everything into the binary with the embedded_assets feature.
sidebar:
  order: 16
---

**Goal:** serve files (images, CSS, JS, a compiled SPA bundle) directly from Loco, either from disk or embedded into the compiled binary.

This assumes a working app. For the full knob table, see the `static` and `spa` entries in the [Middleware catalog reference](/docs/reference/middleware).

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

A frontend build (Vite, webpack, …) gets its own middleware, `spa`, so it can sit next to the `static` folder and your own routes. It is the Rails setup of a `public/` file server plus a catch-all route that renders the app shell, in one config block:

```yaml
server:
  middlewares:
    spa:
      enable: true
      uri: "/app"
      path: "frontend/dist"
```

What it serves, for a build folder with `index.html`, `favicon.ico` and `assets/app-3f2a1c.js`:

| Request | Response |
|---|---|
| `GET /app/assets/app-3f2a1c.js` | the file, `Cache-Control: public, max-age=31536000, immutable` |
| `GET /app/favicon.ico` | the file, `Cache-Control: no-cache` — it keeps its name between builds |
| `GET /app/posts/42` (no such file, no extension) | `index.html`, `Cache-Control: no-cache` — the client-side router takes over |
| `GET /app` | `index.html`, `Cache-Control: no-cache` |
| `GET /app/assets/old-91bc.js` (no such file) | `404` — a stale bundle reference fails instead of being parsed as HTML |
| `GET /app/users/jane.doe` (no such file) | `404` — the last segment has a dot, so it counts as a file, not a client route |
| `GET /app/docs` (a folder in the build) | `index.html` — folders are never listed or redirected |
| `POST /app/anything` | `404` |

The cache split follows how bundlers name their output: Vite writes its built files to `assets/` with a content hash in their names, so a new build produces new URLs and the old ones can be cached forever. `index.html` points at those hashes, so it is always revalidated, and so are files copied as-is from Vite's `public/` folder (`favicon.ico`, `robots.txt`), which keep their names. A `304 Not Modified` gets the same header as the full response would. If your bundler writes hashed files somewhere else, set `cache_control.assets_prefix`.

| Key | Default | Purpose |
|---|---|---|
| `uri` | `/` | where the app is mounted; must start with `/`; `/` serves it for every path no route matched |
| `path` | `frontend/dist` | the build folder |
| `index` | `index.html` | the app shell, relative to `path` |
| `must_exist` | `true` | refuse to boot when `path` or the index file is missing |
| `precompressed` | `false` | serve a `.gz` sibling when the client accepts gzip |
| `cache_control.assets` | `public, max-age=31536000, immutable` | for files under `assets_prefix`; `~` sends no header |
| `cache_control.assets_prefix` | `assets/` | the folder in the build that holds hashed files; `""` counts every non-HTML file |
| `cache_control.html` | `no-cache` | for HTML, including the `index.html` fallback; `~` sends no header |
| `cache_control.other` | `no-cache` | for every other file; `~` sends no header |

The app boots with an error, not a silent default, when `uri` does not start with `/` or a `cache_control` value is not a valid header.

Routes you add in `routes()` under the mount, such as `/app/api/posts`, always win: `spa` only answers what no route matched under its `uri`. The one exception is a route at exactly the mount path (`/app` or `/app/`): it conflicts with the mount and the app panics at boot, so give that route another path. Mounted at `/`, `spa` also replaces the welcome-screen `fallback` — disable that one (section 3), or it shadows the app in development.

Mounted anywhere but `/`, the bundle must know its base path, or `index.html` asks for `/assets/…` instead of `/app/assets/…`. In Vite, set it in `vite.config.ts`:

```ts
export default defineConfig({
  base: "/app/",
});
```

Your client-side router needs the same base (React Router's `basename: "/app"`).

If the SPA is Loco's own clientside mode — a Vite/React frontend in `frontend/`, with TypeScript types generated from your Rust DTOs — this config is already generated for you (mounted at `/`, pointing at `frontend/dist`). See [Build a typed React SPA](/docs/how-to/build-a-spa).

Older apps serve the SPA with `static` and `fallback: "…/index.html"`. That still works, but it sends one `cache_control` for every response, serves `index.html` for a missing `.js` file, and takes the only `static` slot.

The `spa` middleware always reads from disk, including with `embedded_assets` (section 6), so ship the build folder with the binary.

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
curl -I localhost:5150/app/posts/42        # spa: 200, text/html, Cache-Control: no-cache
```

## Next

- [Render server-side views](/docs/how-to/render-views)
- [Add middleware](/docs/how-to/add-middleware)
- [Middleware catalog reference](/docs/reference/middleware)
