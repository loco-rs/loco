+++
title = "Why \"batteries included\"?"
description = "The prime directive behind Loco's design: prefer a built-in or a generator over hand-wiring, and what that buys you."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

Loco's tagline is "Axum with batteries included," and it is meant literally: everything under the hood is standard [Axum](https://crates.io/crates/axum) 0.8 and [Tower](https://crates.io/crates/tower), but almost none of the code you'd normally write to assemble a production web service in Rust — wiring a DB pool into state, picking a logging stack, hand-rolling a queue, choosing a config format — is code you have to write yourself. This page explains the design principle behind that choice, not the mechanics (those live in [Architecture](@/docs/explanation/architecture.md) and the reference pages).

## The prime directive

When a Loco app needs a capability, the framework's bias is:

1. **Reach for a built-in first.** Database access, caching, background jobs, mailing, file storage, view rendering, JWT auth, health checks, a CLI — these already exist, wired into `AppContext` and toggled from YAML.
2. **Reach for a generator second.** `cargo loco generate` scaffolds the idiomatic shape of a model, controller, worker, mailer, task, or full CRUD scaffold. The generated code is not a black box — it's a starting point you own and edit.
3. **Hand-wire only as a last resort**, and when you do, Loco gives you a small number of well-defined seams to do it safely — `Hooks`, `Initializer`, `SharedStore`, `before_routes`/`after_routes` — rather than forcing you to fork the framework or reassemble `main()` from scratch.

This ordering is the single idea that explains most of what looks, from a plain-Axum perspective, like "magic": it isn't magic, it's a library of pre-wired decisions with an escape hatch at every layer.

## What "hand-wiring" looks like without it

A typical Axum service starts every project by re-deciding things that have already been decided a thousand times: which connection-pool settings, which logging crate, how to get the DB handle into every handler, how config and secrets flow in, how a background job survives a restart. None of these decisions are hard, but making them **again**, per project, is where hours disappear and where inconsistency creeps in between a team's services.

Loco's [`coming-from-axum`](@/docs/explanation/coming-from-axum.md) page walks through this delta concretely (pool setup, `AddExtensionLayer` state wiring, `env_logger` vs `tracing`, `main.rs` assembly) for a real reference app. The short version: every one of those steps becomes either a YAML key or a generator invocation in Loco, and the underlying Axum `Router`/`State`/extractor model is unchanged — so nothing about Axum's own learning curve is hidden from you.

## What Loco integrates

Batteries, concretely, means the following are already implemented, tested, and reachable from `AppContext` or a `Hooks` default, rather than left as an exercise:

- **Routing & the request pipeline** — `AppRoutes`/`Routes` compile down to a real `axum::Router<AppContext>`; a documented, ordered stack of middleware (payload limits, CORS, compression, timeouts, security headers, request IDs, a static file server, a dev-mode fallback page, and more) is available with a config flip rather than a `tower::Layer` you write by hand. See [Architecture](@/docs/explanation/architecture.md) and the [middleware catalog](@/docs/reference/middleware.md).
- **Data & persistence** — Sea-ORM 2.0 entities, migrations, and a query/pagination layer are generated from a compact field-type DSL (`cargo loco generate model ...`), not written by hand column-by-column.
- **Background processing** — one `BackgroundWorker` trait and one `perform_later` call site work unmodified against three interchangeable queue backends (Redis, Postgres, SQLite); see [The background-processing model](@/docs/explanation/background-processing-model.md).
- **Scheduling & tasks** — a cron-like scheduler (English or cron syntax) and ad-hoc CLI-invokable tasks, both driven from the same `Tasks`/`Hooks` registration points, no separate process supervisor to build.
- **Caching, storage, mail** — a `Cache` with in-memory/Redis/null backends, a multi-driver `Storage` abstraction (local/memory/S3/Azure/GCS) with single/mirror/backup strategies, and a `Mailer` with SMTP/STARTTLS/implicit-TLS and a stub-for-tests mode — each a field on `AppContext`, each swappable by config or feature flag rather than by rewriting call sites.
- **Views** — server-rendered Tera templates, JSON views, or a single-binary `embedded_assets` build, chosen without touching controller code. See [Views and assets](@/docs/explanation/views-and-assets.md).
- **Auth & security** — JWT (HS512 by default, multi-location token extraction) and API-key extractors implementing the same `FromRequestParts` pattern as everything else in Axum.
- **Operability** — structured `tracing` logging with sane third-party filtering out of the box, `/_ping`/`/_health`/`/_readiness` endpoints, a `cargo loco doctor` diagnostic command, and a `cargo loco routes`/`middleware` introspection CLI.
- **Configuration** — one typed `Config` struct, one environment-resolution rule, one file-precedence rule, and Tera's own `get_env` for secrets — see [The configuration model](@/docs/explanation/configuration-model.md).

None of this requires a plugin marketplace or a runtime registry: it's all compiled into `loco-rs` behind Cargo feature flags (see the [feature-flags reference](@/docs/reference/feature-flags.md)), so an app only pays for what it turns on.

## The corollary: escape hatches, not walls

"Batteries included" only works as a philosophy if it doesn't become "batteries mandatory." Every built-in in Loco has a documented seam for replacing or bypassing it:

- Don't like the default middleware stack? Override `Hooks::middlewares` and return your own `Vec<Box<dyn MiddlewareLayer>>`.
- Want a raw Axum router mounted verbatim? `Hooks::before_routes`/`after_routes` hand you a real `axum::Router` to mutate directly — the [Coming from Axum](@/docs/explanation/coming-from-axum.md) page shows this as the literal drop-in path for existing Axum code.
- Need a service that isn't a first-class `AppContext` field (a third-party API client, a feature-flag SDK)? `AppContext.shared_store` is a type-keyed DI container built for exactly that — see [AppContext and dependency injection](@/docs/explanation/appcontext-and-di.md).
- Want to own the tracing/logging stack yourself? Return `Ok(true)` from `Hooks::init_logger` and Loco steps aside.
- Want a different view engine than Tera? Implement `ViewRenderer` and swap it in via an `Initializer`.

This is the same shape as Rails' "convention over configuration," reframed for a language where a global mutable app instance isn't an option: Loco supplies the convention as a compiled-in default, and the configuration/override points are explicit, typed, and ordered — not implicit and discoverable only by reading source. The rest of this Explanation cluster works through each of those seams — boot lifecycle, DI, config loading, background jobs, views, and the Axum relationship — in more depth.
