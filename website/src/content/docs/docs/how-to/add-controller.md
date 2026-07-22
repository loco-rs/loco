---
title: Add a controller
description: Generate a controller, wire up its routes and handlers, and mount it under a prefix or nested path.
sidebar:
  order: 10
---

**Goal:** add a new HTTP endpoint group to your Loco app — generated, or written by hand — and get it showing up in `cargo loco routes`.

This guide assumes a working Loco app (`cargo loco start` runs). For the full `Routes`/`AppRoutes` API and the exhaustive `Hooks` surface, see the [Hooks reference](/docs/reference/hooks).

## 1. Generate a controller

```sh
cargo loco generate controller <NAME> [ACTION ...] (--api|--html|--htmx)
```

One of `--api`, `--html`, or `--htmx` is **required** — there is no default kind; omitting all three is a hard error. Additional positional arguments become extra actions (handler functions + routes) alongside the default `index`.

```sh
cargo loco generate controller notes list get --api
```

This:

- creates `src/controllers/notes.rs` with an `index` handler plus one handler per extra action (`list`, `get`), each returning `format::empty()` as a starting point
- adds `pub mod notes;` to `src/controllers/mod.rs`
- injects `.add_route(controllers::notes::routes())` into your `routes()` implementation in `src/app.rs` — no manual wiring needed
- generates a matching test file under `tests/requests/`

The generated `routes()` function looks like this:

```rust
// src/controllers/notes.rs
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/notes/")
        .add("/", get(index))
        .add("list", get(list))
        .add("get", get(get))
}
```

Edit the handler bodies and route methods (`get`/`post`/`put`/`delete`, etc.) to fit your endpoint. `--html`/`--htmx` generate the same shape plus `src/views/<name>.rs` view stubs and templates under `assets/views/` — see [Render server-side views](/docs/how-to/render-views) for what to do with those.

## 2. Confirm the routes are registered

```sh
cargo loco routes
```

```sh
[GET] /_ping
[GET] /_health
[GET] /_readiness
[GET] /api/notes/
[GET] /api/notes/list
[GET] /api/notes/get
```

If your new routes don't appear, check that `src/app.rs`'s `routes()` implementation calls `.add_route(controllers::notes::routes())` (the generator does this for you, but double-check after a manual edit or merge conflict).

## 3. Write a controller by hand (no generator)

Sometimes you want a controller without a generator scaffold — e.g. a small internal endpoint.

1. Create `src/controllers/example.rs`:

   ```rust
   use loco_rs::prelude::*;

   async fn hello() -> Result<Response> {
       format::text("hello")
   }

   async fn echo(Json(body): Json<serde_json::Value>) -> Result<Response> {
       format::json(body)
   }

   pub fn routes() -> Routes {
       Routes::new()
           .add("/", get(hello))
           .add("/echo", post(echo))
   }
   ```

2. Declare the module in `src/controllers/mod.rs`:

   ```rust
   pub mod example;
   ```

3. Register its routes in `src/app.rs`'s `Hooks::routes`:

   ```rust
   fn routes(_ctx: &AppContext) -> AppRoutes {
       AppRoutes::with_default_routes()
           .add_route(controllers::example::routes())
   }
   ```

`AppRoutes::with_default_routes()` also mounts the built-in `/_ping`, `/_health`, `/_readiness` monitoring endpoints.

## 4. Prefix a whole controller

`Routes::prefix` scopes every route added to that `Routes` instance:

```rust
pub fn routes() -> Routes {
    Routes::new()
        .prefix("notes")
        .add("/", get(list))
        .add("/{id}", get(get_one))
}
```

## 5. Prefix a whole app (or a group of controllers)

`AppRoutes::prefix` applies to every controller added after it:

```rust
fn routes(_ctx: &AppContext) -> AppRoutes {
    AppRoutes::with_default_routes()
        .prefix("/api")
        .add_route(controllers::notes::routes())
        .add_route(controllers::users::routes())
}
```

## 6. Nest routes under an additional path segment

Use `nest_prefix` to append another path segment to the *current* prefix for routes added afterward, or `nest_route`/`nest_routes` to scope a prefix to just the routes passed in (without touching the running prefix):

```rust
fn routes(_ctx: &AppContext) -> AppRoutes {
    let v1_notes = Routes::new().add("/", get(|| async { "notes v1" }));

    AppRoutes::with_default_routes()
        .prefix("api")
        .add_route(controllers::auth::routes())
        // only these routes get the extra `v1` segment: /api/v1/...
        .nest_route("v1", v1_notes)
}
```

`Routes::nest` (on a `Routes` value, not `AppRoutes`) does the same job when you're composing route groups before returning them from a controller's `routes()` function — handy for merging several sub-resources with `Routes::merge`/`merge_all` and then nesting the result once:

```rust
let user_routes = Routes::new()
    .add("/users", get(list_users))
    .add("/users", post(create_user));
let product_routes = Routes::new().add("/products", get(list_products));

let api_routes = Routes::new().merge(user_routes).merge(product_routes);

Routes::new()
    .add("/health", get(|| async { "ok" }))
    .nest("/api", api_routes);
// -> GET /health, GET /api/users, POST /api/users, GET /api/products
```

## 7. Apply a `tower::Layer` to just one controller or route

`Routes::layer` attaches a `tower::Layer` (rate limiting, custom auth, tracing, etc.) to every handler in that `Routes` value only — for middleware that should run on *every* route, see [Add middleware](/docs/how-to/add-middleware) instead.

```rust
// src/controllers/notes.rs
pub fn routes() -> Routes {
    Routes::new()
        .prefix("notes")
        .add("/", get(list).layer(my_tower_layer()))
}
```

## Verify

```sh
cargo loco routes
cargo test --test requests_notes   # if the generator produced tests/requests/notes.rs
```

A `curl` against the new path should return your handler's response:

```sh
curl -s localhost:5150/api/notes/
```

## Next

- [Validate requests](/docs/how-to/validate-requests)
- [Respond with different formats](/docs/how-to/respond-formats)
- [Handle errors](/docs/how-to/handle-errors)
