+++
title = "Architecture: the request lifecycle"
description = "How a Loco app boots, how AppContext gets built, and how a request travels through routes, middleware, and back out — and why the middleware order is LIFO."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

A Loco app has two distinct timelines that are worth keeping separate in your head: **boot** (runs once, assembles everything the app needs) and **request handling** (runs per HTTP request, through a fixed pipeline). This page walks both, and explains the one piece of ordering that surprises almost everyone the first time: middleware runs in the *reverse* of the order you list it in.

## Boot: from `StartMode` to a running app

Everything starts from `src/boot.rs`, driven by the `Hooks` trait your `App` implements (the exhaustive method-by-method reference is [Hooks trait](@/docs/reference/hooks.md)). At a high level:

```text
cargo loco start
   │
   ▼
Hooks::load_config(env)            → Config            (default: env.load())
   │
   ▼
create_context::<App>(env, config) → AppContext          (db, mailer, queue, cache, storage wired up)
   │                                  Hooks::after_context(ctx) can rewrite ctx here
   ▼
db::converge + bgworker::converge  (migrations / queue setup, if applicable)
   │
   ▼
run_app::<App>(mode, ctx)          → BootResult
   │   ├─ Hooks::before_run(&ctx)
   │   ├─ Hooks::initializers(&ctx) → before_run() on each Initializer
   │   ├─ Hooks::routes(&ctx)       → AppRoutes
   │   ├─ Hooks::before_routes / after_routes(router, &ctx)
   │   ├─ Hooks::middlewares(&ctx)  → Vec<Box<dyn MiddlewareLayer>>
   │   └─ after_routes() on each Initializer
   ▼
start::<App>(boot, server_config)  → binds the socket, spawns the scheduler if requested,
                                      calls Hooks::serve(...), prints the banner
```

Each `Hooks` method in that chain has a sensible default (see the reference for the exact signatures and defaults) — a minimal `App` only needs to implement `app_name`, `boot`, `routes`, `connect_workers`, `register_tasks`, and (with a database) `truncate`/`seed`. Everything else — logging setup, config loading, the middleware stack, initializer wiring — is a provided method you override only when you need to change it.

### `StartMode`: what actually runs in this process

`boot()` receives a `StartMode` that determines which of the app's subsystems are live in *this* process:

| Mode | Server | Worker | Scheduler |
|---|---|---|---|
| `ServerOnly` | yes | no | no |
| `ServerAndWorker` | yes | yes (same process) | no |
| `ServerAndScheduler` | yes | no | yes |
| `WorkerOnly { tags }` | no | yes, filtered by tag | no |
| `WorkerAndScheduler { tags }` | no | yes, filtered by tag | yes |
| `All` | yes | yes | yes |

`StartMode` exists because "the web server" and "the thing that drains the job queue" don't have to be the same OS process — in fact for anything beyond a single-dyno deployment you usually *want* them separate, so you can scale workers and the HTTP tier independently. `cargo loco start --worker`, `--server-and-worker`, `--scheduler`, and `--all` map directly onto these variants (`cargo loco start` alone is `ServerOnly`). The worker only actually runs if `workers.mode` in config is `BackgroundQueue` — see [The background-processing model](@/docs/explanation/background-processing-model.md) for why.

### `AppContext` is assembled once, here

`create_context` is the one place `AppContext`'s eight fields (`environment`, `db`, `queue_provider`, `config`, `mailer`, `storage`, `cache`, `shared_store`) get their real values, before `Hooks::after_context` gets a final chance to post-process the struct (e.g. to stash a custom service into `shared_store`). Everything downstream — routing, middleware, handlers, background workers, tasks, the scheduler — receives the *same* `AppContext` value (it's cheaply `Clone`), which is why it's the natural place to reach for shared state. See [AppContext and dependency injection](@/docs/explanation/appcontext-and-di.md) for the full story on that struct and its `shared_store` extensibility slot.

## Request handling: the onion

Once boot finishes, `AppRoutes::to_router` has compiled your routes plus the middleware stack into one real `axum::Router<AppContext>`. A request's journey through it looks like this:

```text
   inbound request
        │
        ▼
┌───────────────────────────────────────────┐
│ powered_by            (outermost — first) │
│  ┌────────────────────────────────────┐   │
│  │ fallback (non-prod)                │   │
│  │  ┌──────────────────────────────┐  │   │
│  │  │ request_id                   │  │   │
│  │  │  ┌────────────────────────┐  │  │   │
│  │  │  │ logger                 │  │  │   │
│  │  │  │  ┌──────────────────┐  │  │  │   │
│  │  │  │  │ … cors, etag,    │  │  │  │   │
│  │  │  │  │   compression …  │  │  │  │   │
│  │  │  │  │  ┌────────────┐  │  │  │  │   │
│  │  │  │  │  │limit_payload│  │  │  │  │   │
│  │  │  │  │  │ (innermost)│  │  │  │  │   │
│  │  │  │  │  │  ┌──────┐  │  │  │  │  │   │
│  │  │  │  │  │  │handler│  │  │  │  │  │   │
│  │  │  │  │  │  └──────┘  │  │  │  │  │   │
│  │  │  │  │  └────────────┘  │  │  │  │   │
│  │  │  │  └──────────────────┘  │  │  │   │
│  │  │  └────────────────────────┘  │  │   │
│  │  └──────────────────────────────┘  │   │
│  └────────────────────────────────────┘   │
└───────────────────────────────────────────┘
        │
        ▼
    response, unwinding back out the same layers
```

### Why the order is LIFO

`AppRoutes::to_router` builds this onion by calling `app.layer(...)` once per middleware, in the order `default_middleware_stack` lists them (`limit_payload` first, `powered_by` last). Axum's `Router::layer` wraps the *existing* router with each new layer as the new **outermost** layer. The consequence, stated directly in the framework's own source comment:

> "the LAST middleware is the FIRST to meet the outside world (a user request starting), or 'LIFO' order" — `src/controller/app_routes.rs`

So the coding/config order and the runtime order are opposites: routes are added first (they become the innermost core of the onion — the thing every layer eventually wraps), and the *last* middleware added (`powered_by`, at the bottom of the default list) is the *first* thing an inbound request actually passes through. `request_id` is deliberately near the end of the list (so it's near the *outside* at runtime) precisely because every request needs its ID assigned as early in its life as possible.

This matters practically whenever you reach for `Routes::layer(...)` to attach a `tower::Layer` to one controller, or override `Hooks::middlewares` to reorder the default stack — get the direction backwards and a middleware that's supposed to run before authentication ends up running after it. The full ordered list, with each middleware's config key and default-enabled state, is in the [middleware catalog reference](@/docs/reference/middleware.md); [extras/pluggability.md](@/docs/extras/pluggability.md) shows how to hand-write a `tower::Layer` middleware of your own that participates in the same onion.

## Where this leaves the handler

By the time your handler runs, it's just a normal Axum handler function taking normal Axum extractors (`State<AppContext>`, `Json<T>`, `Path<T>`, and so on — nothing Loco-specific is required). The response side of the onion is symmetric: your `impl IntoResponse` (or Loco's `format::` helpers) produces a `Response`, which then unwinds back out through the same middleware stack in reverse, each layer getting a chance to post-process it (compression, headers, logging the outcome) before it leaves the process.

For the mechanics of how routes get their axum `Router<AppContext>` shape (`AppRoutes`, `Routes`, prefixing, nesting) see [`the-app/controller.md`](@/docs/the-app/controller.md); for how this whole model maps onto plain Axum concepts you already know, see [Coming from Axum](@/docs/explanation/coming-from-axum.md).
