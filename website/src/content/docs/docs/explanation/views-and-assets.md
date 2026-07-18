---
title: Views and assets
description: SSR with Tera vs. serving a SPA vs. embedding everything into the binary — and how one feature flag coordinates a swap across two subsystems at once.
sidebar:
  order: 6
---

"How does this response get to the browser" has three different shapes in a Loco app — server-rendered HTML, a JSON API behind a separately-built SPA, or a fully embedded single binary — and Loco lets you pick without changing how controllers work. This page explains the model behind that choice. For the how-to of writing a specific view or wiring the static middleware, see [Render server-side views](@/docs/how-to/render-views.md); for the exhaustive middleware/config keys, see the [middleware catalog](@/docs/reference/middleware.md) and [feature flags reference](@/docs/reference/feature-flags.md).

## The separation controllers don't have to care about

Loco keeps the traditional split of responsibilities — a controller parses the request and calls into models; a *view* is responsible only for shaping the final response — and makes that split concrete with one trait:

```rust
pub trait ViewRenderer {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String>;
}
```

A controller never talks to Tera, or to any specific templating engine, directly — it takes a `v: impl ViewRenderer` (typically via the `ViewEngine<E>` extractor) and calls `format::render().view(&v, "home/hello.html", data!({..}))`. The engine behind that trait is decided once, at the `Initializer` level — swap `ViewEngine<TeraView>` for `ViewEngine<YourEngine>` and every existing call site keeps compiling, because it was only ever coupled to the trait, not to Tera specifically. This is the same escape-hatch pattern described in [Why batteries included](@/docs/explanation/why-batteries-included.md): Tera is the built-in, `ViewRenderer` is the seam you use if you need something else.

For pure JSON APIs, "the view" can be as simple as a `#[derive(Serialize)]` struct shaped by hand and returned via `format::json(..)` — no template engine in the loop at all. Most real apps mix both: JSON views for an API surface, Tera views for a handful of server-rendered pages (an admin panel, a marketing page, an email-verification landing page).

## Three deployment shapes, one set of controller code

### Server-side rendering (Tera)

The default shape: `TeraView` reads templates from `assets/views/**/*.html` on disk at request time, alongside static files served from `assets/static/` through the `static` middleware. This is the natural choice when the app itself renders the HTML the browser gets — templates can be edited without a rebuild (`cargo loco start` picks up a changed `.html` file on the next request), which matters during active UI development.

### Client-side rendering (SPA)

Here Loco's job shrinks to two things: serve a JSON API, and serve the SPA's *built* static assets (the `dist/`-style output of a separate frontend build) through the same `static` middleware, usually with a fallback to `index.html` for client-side routing. There's no `TeraView` in the picture for the API surface at all — the split between "backend serves data" and "frontend owns rendering" is total, and Loco's role is just: API controllers, plus a static file server pointed at wherever the frontend build lands.

### `embedded_assets`: one flag, two subsystems swapped together

`embedded_assets` is a build-time Cargo feature that changes *where the bytes come from* without changing a single controller or view call:

```toml
loco-rs = { version = "...", features = ["embedded_assets"] }
```

With it enabled, the entire `assets/` directory — templates *and* static files — is scanned at compile time and embedded directly into the binary. What makes this worth calling out as a distinct architectural idea, rather than just "a smaller deployment," is that it isn't one subsystem being swapped: **both** the Tera view engine and the static-assets middleware are simultaneously replaced with embedded-reading variants (`views::engine_embedded` in place of `views::engine`, `static_assets_embedded` in place of `static_assets`) behind the same `#[cfg(embedded_assets)]` gate. One flag flips two independently-registered subsystems in lockstep, so a template lookup and a static-file request both resolve against the same in-binary asset table rather than one reading disk and the other reading memory. From application code, nothing changes — the `ViewEngine`/`ViewRenderer` and static-middleware config keys are identical either way.

The trade-off is the mirror image of SSR's live-editing convenience: a single-binary deploy with atomic code/asset updates and no filesystem asset directory to manage in production, at the cost of a full recompile for any asset change and a larger binary. Projects often use plain filesystem assets in development (fast iteration) and flip `embedded_assets` on for release builds (simpler deployment) — the same controller and view code runs unmodified in both.

## Why this is one flag and not per-subsystem toggles

It would be possible to let the view engine and the static middleware be embedded independently — but that would create deployment shapes where templates are baked into the binary while static files are read from disk (or vice versa), with no clear operational benefit and a real risk of the two drifting (a rebuild updates embedded templates but not the separately-deployed static folder, or the reverse). Coupling both under one feature flag makes "embedded" a single, coherent deployment mode rather than a matrix of partial states to reason about — consistent with the broader theme in [Why batteries included](@/docs/explanation/why-batteries-included.md) of collapsing a class of decisions into one well-tested default, with a clearly labeled way to opt out (here, just don't enable the feature).

See [Render server-side views](@/docs/how-to/render-views.md) for the concrete `assets/` directory layout, writing a Tera view and its controller wiring, and swapping in a custom `ViewRenderer`; see the [feature flags reference](@/docs/reference/feature-flags.md) for `embedded_assets`'s place in the full flag matrix, and the [middleware catalog](@/docs/reference/middleware.md#8-static) for the `static` middleware's config keys.
