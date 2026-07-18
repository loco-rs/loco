---
title: Coming from Axum
description: Loco is Axum 0.8 with pre-wired decisions on top, not a replacement for it — how extractors, State, and the Router map across, and what Loco actually adds.
sidebar:
  order: 7
---

If you already know [Axum](https://crates.io/crates/axum), you already know most of Loco — the framework compiles down to a real `axum::Router<AppContext>`, uses the same `FromRequestParts`/`FromRequest` extractor model, and pins Axum 0.8. This page is about the delta: what Loco pre-wires on top, and how the concepts you already have a mental model for (extractors, `State`, the `Router`) map onto Loco's names for the same things, plus the mechanics of moving a real Axum codebase over; for the request lifecycle these concepts sit inside, see [Architecture](@/docs/explanation/architecture.md).

## The core claim: nothing is hidden, a lot is pre-decided

Loco is not a new web framework with an Axum-shaped API — it *is* Axum, with a layer of default decisions and a `Hooks` trait that assembles them consistently across every app that uses it. Every extractor you already know still works unmodified; the state type is just a specific struct (`AppContext`) instead of whatever ad-hoc struct you'd have hand-rolled; and the middleware you'd have written as `tower::Layer`/`Service` impls yourself either already exists as a config-toggleable built-in, or you write it exactly the way you would in plain Axum and attach it the same way. [Why batteries included](@/docs/explanation/why-batteries-included.md) covers the philosophy; this page covers the mechanical mapping.

## Concept mapping

| Axum concept | Loco equivalent | What changed |
|---|---|---|
| Your own `main()` assembling the router, state, and `axum::serve(...)` | `Hooks::boot` → `create_app`/`create_context`, `Hooks::serve` (default calls `axum::serve` for you) | You describe *what* to wire (routes, workers, tasks) via `Hooks`; Loco's boot sequence (see [Architecture](@/docs/explanation/architecture.md)) does the assembling. `cargo loco start` replaces a hand-written `main.rs` entirely — a generated app doesn't need one. |
| A hand-rolled `ApiContext` struct + `AddExtensionLayer`/`State` | `AppContext` (8 fields: `environment`, `db`, `queue_provider`, `config`, `mailer`, `storage`, `cache`, `shared_store`), `#[derive(FromRef)]` | Same idea (one struct, threaded as Axum `State`) but pre-built with the pieces almost every service needs, and `FromRef`-derived so you can extract a single field (`State<DatabaseConnection>`) instead of always the whole context. See [AppContext and dependency injection](@/docs/explanation/appcontext-and-di.md). |
| `Router::new().route("/x", get(handler))` | `Routes::new().add("/x", get(handler))`, collected into `AppRoutes` | `Routes`/`AppRoutes` are a thin builder over the same `MethodRouter`/`Router` types — `add`, `prefix`, `nest_route`, `merge`, and `layer` all compile down to the Axum calls you'd write by hand, plus route metadata (`cargo loco routes` listing) that a plain Axum `Router` can't give you back. |
| `.layer(SomeTowerLayer::new())` on a router or route | `Routes::layer(..)` (per-route) or `Hooks::middlewares` (app-wide) | Identical `tower::Layer`/`Service` code — Loco doesn't wrap or reinterpret Tower's traits. The app-wide default stack (CORS, compression, timeouts, etc.) is config-toggled rather than hand-attached; see the [middleware catalog](@/docs/reference/middleware.md) and its LIFO ordering note in [Architecture](@/docs/explanation/architecture.md#why-the-order-is-lifo). |
| `Extension<T>` / custom `FromRequestParts` impls for app-specific data | `State<AppContext>` field extraction, or `SharedStore<T>` for anything not already a field | See [AppContext and dependency injection](@/docs/explanation/appcontext-and-di.md) for when to reach for which. |
| `dotenv` + manual env var parsing in `main` | Typed `Config` loaded from `config/{env}.yaml`, Tera's `get_env` for env-var interpolation | See [The configuration model](@/docs/explanation/configuration-model.md). |
| `env_logger`/`tracing_subscriber` set up by hand | `logger::init` (default), or return `Ok(true)` from `Hooks::init_logger` to opt out entirely and own it yourself | Loco's default filters out third-party log noise you didn't ask for while still using plain `tracing` underneath — any crate emitting `tracing` events shows up the same way it would under a hand-rolled subscriber. |
| A router you already have, that you don't want to restructure | Return it untouched from `Hooks::before_routes`/`after_routes` | This is the literal drop-in path: mount an existing `axum::Router` as-is and keep every extractor and handler signature you already wrote. |

## What "drop-in compatible" actually buys you

Because routing metadata is optional rather than mandatory, you have a genuine choice at the boundary between "paste in existing Axum code" and "get Loco's introspection for free":

- Return your existing router verbatim from `after_routes(router, _ctx)` — zero changes to handler signatures, zero changes to how routes are declared, full compatibility, but `cargo loco routes` won't know about those routes (Axum doesn't expose method/path metadata off a live `Router`, which is precisely the gap `Routes`/`AppRoutes` exist to close).
- Rewrite route declarations from `Router::new().route(path, method(handler))` to `Routes::new().add(path, method(handler))` — this is a mechanical, same-shape edit (the handler itself, its extractors, and its return type are untouched) — and get route listing, prefixing/nesting helpers, and per-route `tower::Layer` attachment (`Routes::layer`) back.

Most apps end up doing the second for their own controllers and the first only for vendored or generated routers they don't want to touch.

## What Loco adds that plain Axum genuinely doesn't have

Everything in the mapping table above is a *reframing* of something Axum already gives you. The following are not reframings — they're capabilities with no direct Axum equivalent, because they're above the HTTP layer:

- A background-job system (`BackgroundWorker`, three interchangeable queue backends) — see [The background-processing model](@/docs/explanation/background-processing-model.md).
- A cron-like scheduler and a CLI task runner, both driven from the same app.
- Database access via Sea-ORM with a generator that scaffolds models/migrations from a field-type DSL.
- A `cargo loco` CLI: `routes`, `middleware --config`, `doctor`, `jobs`, `db`, `generate`.
- Structured JWT/API-key auth extractors, a cache abstraction, and a multi-driver storage abstraction, each swappable by config rather than by code change.

These are the parts of "batteries included" that live outside the request/response cycle Axum itself models — which is also why they're covered by their own pages in this cluster rather than in this one's extractor/router mapping.

## A note on versions

Loco 1.0 tracks Axum 0.8 and targets Rust edition 2024 (the `loco-rs` crate itself; apps generated by `loco new` currently still default to edition 2021 in their own `Cargo.toml` — bump it yourself if you want 2024-edition semantics, such as the `unsafe`-required `std::env::set_var`, in your own app code). If you're moving code from an Axum service built against an older Axum release, the usual Axum 0.7→0.8 migration notes apply on top of everything above — nothing here changes because of Loco.
