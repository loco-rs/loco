---
title: AppContext and dependency injection
description: Why AppContext is the one piece of shared state every handler sees, and how SharedStore lets you extend it without forking the framework.
sidebar:
  order: 3
---

Rust doesn't let you reach for a mutable global app instance the way Rails or Django can — there's no ambient `current_app` to mutate from anywhere. Loco's answer to "how does a handler get at the database, the cache, the mailer, my own service client?" is a single, cheaply-cloneable struct threaded through the whole app: `AppContext`. This page explains why that struct has the shape it does, and how `SharedStore` extends it to things Loco itself doesn't know about.

## `AppContext` as the one shared-state object

Every handler, background worker, task, and scheduled job in a Loco app receives the same `AppContext` value — assembled once at boot (see [Architecture](/docs/explanation/architecture)) and cloned cheaply wherever it's needed, because most of its fields are already `Arc<...>`-wrapped or otherwise cheap to clone. It carries eight fields:

```rust
#[derive(Clone, FromRef)]
#[non_exhaustive]
pub struct AppContext {
    pub environment: Environment,
    #[cfg(feature = "with-db")]
    pub db: DatabaseConnection,
    pub queue_provider: Option<Arc<bgworker::Queue>>,
    pub config: Config,
    pub mailer: Option<EmailSender>,
    pub storage: Arc<Storage>,
    pub cache: Arc<cache::Cache>,
    pub shared_store: Arc<SharedStore>,
}
```

The full field-by-field reference — types, feature gates, purpose — lives in [AppContext & prelude](/docs/reference/app-context). What's worth explaining here is the *design*, not the field list:

- **One struct, not seven services.** Rather than injecting the DB pool, the cache, the mailer, storage, and the queue as five separate pieces of Axum state, Loco bundles them into one `AppContext` and derives `FromRef` on it. That derive is what lets a handler ask for exactly the piece it needs — `State<DatabaseConnection>` or `State<Arc<cache::Cache>>` — while a background worker or the boot sequence can still ask for the whole thing. You get the ergonomics of narrow, single-purpose extraction without the boilerplate of hand-writing `FromRef` impls for every field.
- **`db` is the only field that's compiled away, not just empty.** Every other field degrades gracefully when unconfigured (`None` for `mailer`/`queue_provider`, a `Null` driver for `cache`/`storage`) — an app with no mailer configured still has a `mailer: Option<EmailSender>` field, just set to `None`. `db` is different: with the `with-db` feature off, the field doesn't exist on the struct at all, which is a compile-time way of saying "this deployment shape genuinely has no database," rather than a runtime `Option` a caller could forget to check.
- **It's the same value everywhere.** Because `create_context` builds `AppContext` exactly once at boot and every subsystem downstream — routing, middleware, handlers, `connect_workers`, tasks, the scheduler — receives that same value, there's no risk of a handler and a background job disagreeing about which DB pool or cache instance is "the real one." This is also why `Hooks::after_context(ctx: AppContext) -> Result<AppContext>` (which runs immediately after assembly, before routes are built) is the one hook that can rewrite the context itself — it's your last and only chance to add something to it before it's handed out everywhere.
- **It's `#[non_exhaustive]`, so it's built through a builder.** Marking the struct `#[non_exhaustive]` means app code can neither write an `AppContext { .. }` literal nor use functional-update syntax — which is what makes adding a field in a later Loco release a non-breaking change. In its place, `AppContext::builder(..)` takes the required components as arguments and defaults the optional ones to no-op providers, and `ctx.into_builder()` turns an existing context back into a builder with every component carried over. The second one is what `after_context` should use: rebuilding from scratch would silently discard the mailer, queue provider, cache and shared store that boot had already put in place, whereas a round-trip replaces one component and keeps the rest.

## The gap `AppContext`'s fixed fields can't fill

`AppContext`'s eight fields cover what *every* Loco app needs. They obviously can't cover what *your* app needs — a third-party API client, a feature-flag SDK handle, an app-specific cache of precomputed data. Two options exist, and they aren't in tension, they're the same design carried into two different lifecycles:

- **`Initializer`** (see [Add middleware](/docs/how-to/add-middleware)) is the *install-time* extension point: a trait with `before_run`, `after_routes`, and `check` hooks, used to wire a whole piece of infrastructure into the app (register an Axum `Extension`, mount a session layer, install a doctor health check).
- **`SharedStore`** is the *storage* extension point: a place to actually hold a value of a type Loco has never heard of, so it can be read back out in a handler, a worker, or anywhere else `AppContext` reaches.

In practice they compose: you typically construct the value you want to share and call `ctx.shared_store.insert(..)` from inside `Hooks::after_context` (the same hook that runs once, right after the context is built), then read it back with the extractor below.

## `SharedStore`: a type-keyed DI container

`AppContext.shared_store: Arc<SharedStore>` is a small, concurrent, heterogeneous store — internally a `DashMap` keyed by `TypeId`, so it can hold one value of any number of distinct `'static + Send + Sync` types at once. Its API is deliberately minimal:

| Method | What it does |
|---|---|
| `insert<T>(&self, val: T)` | Store (or overwrite) the value for type `T`. |
| `get<T: Clone>(&self) -> Option<T>` | Fetch a *cloned* copy of `T`, if present. |
| `get_ref<T>(&self) -> Option<RefGuard<'_, T>>` | Fetch a borrowed `Deref<Target = T>` guard — for types that aren't `Clone`, or when a clone would be wasteful. |
| `remove<T>(&self) -> Option<T>` | Take the value back out. |
| `contains<T>(&self) -> bool` | Check presence without touching the value. |

### Reading it back: two paths, and a naming hazard to watch for

There are two distinct `SharedStore` types reachable from `loco_rs::prelude`, and confusing them is the single easiest mistake to make with this feature:

- **`loco_rs::app::SharedStore`** — the container type above, held as `ctx.shared_store`.
- **`loco_rs::controller::extractor::shared_store::SharedStore<T>(pub T)`** — an Axum `FromRequestParts<AppContext>` *extractor*, also re-exported as `SharedStore` from the prelude, that reaches into `ctx.shared_store`, clones out a `T`, and hands it to your handler as a plain argument:

```rust
#[debug_handler]
pub async fn index(
    SharedStore(service): SharedStore<MyClonableService>,
) -> impl IntoResponse {
    tracing::info!("api key: {}", service.api_key);
    format::empty()
}
```

If `T` was never inserted, the extractor rejects the request with `Error::InternalServerError` — which is the right failure mode for "the app forgot to wire something up," as opposed to a client-facing 4xx. For a type that isn't `Clone`, skip the extractor and reach for `ctx.shared_store.get_ref::<T>()` directly off an ordinary `State<AppContext>` extraction instead — you get a reference-counted guard rather than a copy.

## Why this shape, instead of a global registry

The alternative designs are familiar from other ecosystems — a service locator singleton, or a compile-time DI container that resolves a dependency graph. Loco deliberately avoids both:

- A **global mutable singleton** isn't something safe Rust gives you for free, and reaching for `unsafe`/`OnceCell`-style globals to fake one would undermine the exact guarantee (no data races, no invisible mutation from anywhere) that makes Rust worth using for a server in the first place.
- A **compile-time DI framework** (resolving constructor graphs, macro-generated wiring) adds a second configuration language on top of Rust itself, for a problem `AppContext` plus `SharedStore` already solves at the cost of one `insert`/`get` pair.

`SharedStore` is intentionally closer to "a typed, thread-safe `HashMap<TypeId, Box<dyn Any>>` you're handed for free" than a general DI framework — it doesn't manage lifecycles, doesn't resolve dependencies between the things you store, and doesn't enforce a registration order. That's the trade: less power, but no new mental model to learn, and it composes with ordinary Rust ownership rather than working around it. For most apps, "stash a client in `after_context`, extract it with `SharedStore<T>`" is the entire pattern.

See the [AppContext & prelude reference](/docs/reference/app-context) for the exhaustive field/method signatures, and [Add middleware](/docs/how-to/add-middleware) for a related worked example (a custom `MiddlewareLayer`/`Initializer`-style extension wired into the app).
