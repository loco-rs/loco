+++
title = "Axum vs Loco"
description = "Shows how to move from Axum to Loco"
date = 2023-12-01T19:30:00+00:00
updated = 2023-12-01T19:30:00+00:00
draft = false
weight = 5
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++

<div class="infobox">
<b>NOTE: Loco is based on Axum, it is "Axum with batteries included"</b>, and is very easy to move your Axum code to Loco.
</div>

We will study [realworld-axum-sqlx](https://github.com/launchbadge/realworld-axum-sqlx) which is an Axum based app, that attempts to describe a real world project, using API, real database, and real world scenarios as well as real world operability requirements such as configuration and logging.

Picking `realworld-axum-sqlx` apart piece by piece **we will show that by moving it from Axum to Loco, most of the code is already written for you**, you get better best practices, better dev experience, integrated testing, code generation, and build apps faster.

**You can use this breakdown** to understand how to move your own Axum based app to Loco as well. For any questions, reach out [in discussions](https://github.com/loco-rs/loco/discussions) or join our [discord by clicking the green invite button](https://github.com/loco-rs/loco)

## `main`

When working with Axum, you have to have your own `main` function which sets up every component of your app, gets your routers, adds middleware, sets context, and finally, eventually, goes and sets up a `listen` on a socket.

This is a lot of manual, error prone work. 

In Loco you:

* Toggle on/off your desired middleware in configuration
* Use `cargo loco start`, no need for a `main` file at all
* In production, you get a compiled binary named `your_app` which you run


### Moving to Loco

* Set up your required middleware in Loco `config/`

```yaml
server:
  middlewares:
    limit_payload:
      body_limit: 5mb
  # .. more middleware below ..
```

* Set your serving port in Loco `config/`

```yaml
server:
  port: 5150
```

### Verdict

* **No code to write**, you don't need to hand-code a main function unless you have to
* **Best practices off the shelf**, you get a main file best practices uniform, shared across all your Loco apps
* **Easy to change**, if you want to remove/add middleware to test things out, you can just flip a switch in configuration, no rebuild


## Env

The realworld axum codebase uses [dotenv](https://github.com/launchbadge/realworld-axum-sqlx/blob/main/.env.sample), which needs explicit loading in `main`:

```rust
 dotenv::dotenv().ok();
```

And a `.env` file to be available, maintained and loaded:

```
DATABASE_URL=postgresql://postgres:{password}@localhost/realworld_axum_sqlx
HMAC_KEY={random-string}
RUST_LOG=realworld_axum_sqlx=debug,tower_http=debug
```

This is a **sample** file which you get with the project, which you have to manually copy and edit, which is more often than not very error prone.

### Moving to Loco

Loco: use your standard `config/[stage].yaml` configuration, and load specific values from environment using `get_env`


```yaml
# config/development.yaml

# Web server configuration
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: {{% get_env(name="NODE_PORT", default=5150) %}}
```

This configuration is strongly typed, contains most-used values like database URL, logger levels and filtering and more. No need to guess or reinvent the wheel.

### Verdict

* **No coding needed**, when moving to Loco you write less code
* **Less moving parts**, when using Axum only, you have to have configuration in addition to env vars, this is something you get for free with Loco

## Database

Using Axum only, you typically have to set up your connection, pool, and set it up to be available for your routes, here's the code which you put in your `main.rs` typically:

```rust
    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .context("could not connect to database_url")?;
```

Then you have to hand-wire this connection
```rust
 .layer(AddExtensionLayer::new(ApiContext {
                config: Arc::new(config),
                db,
            }))
```

### Moving to Loco

In Loco you just set your values for the pool in your `config/` folder. We already pick up best effort default values so you don't have to do it, but if you want to, this is how it looks like:


```yaml
database:
  enable_logging: false
  connect_timeout: 500
  idle_timeout: 500
  min_connections: 1
  max_connections: 1
```

### Verdict

* **No code to write** - save yourself the dangers of picking the right values for your db pool, or misconfiguring it
* **Change is easy** - often you want to try different values under different loads in production, with Axum only, you have to recompile, redeploy. With Loco you can set a config and restart the process.


## Logging

All around your app, you'll have to manually code a logging story. Which do you pick? `tracing` or `slog`? Is it logging or tracing? What is better?

Here's what exists in the real-world-axum project. In serving:

```rust
  // Enables logging. Use `RUST_LOG=tower_http=debug`
  .layer(TraceLayer::new_for_http()),
```

And in `main`:

```rust
    // Initialize the logger.
    env_logger::init();
```

And ad-hoc logging in various points:

```rust
  log::error!("SQLx error: {:?}", e);
```

### Moving to Loco

In Loco, we've already answered these hard questions and provide multi-tier logging and tracing:

* Inside the framework, internally
* Configured in the router
* Low level DB logging and tracing
* All of Loco's components such as tasks, background jobs, etc. all use the same facility

And we picked `tracing` so that any and every Rust library can "stream" into your log uniformly. 

But we also made sure to create smart filters so you don't get bombarded with libraries you don't know, by default.

You can configure your logger in `config/`

```yaml
logger:
  enable: true
  pretty_backtrace: true
  level: debug
  format: compact
```

### Verdict

* **No code to write** - no set up code, no decision to make. We made the best decision for you so you can write more code for your app.
* **Build faster** - you get traces for only what you want. You get error backtraces which are colorful, contextual, and with zero noise which makes it easier to debug stuff. You can change formats and levels for production.
* **Change is easy** - often you want to try different values under different loads in production, with Axum only, you have to recompile, redeploy. With Loco you can set a config and restart the process.

## Routing

Moving routes from Axum to Loco is actually drop-in. Loco uses the native Axum router.

If you want to have facilities like route listing and information, you can use the native Loco router, which translates to an Axum router, or you can use your own Axum router.


### Moving to Loco

If you want 1:1 complete copy-paste experience, just copy your Axum routes, and plug your router in Loco's `after_routes()` hook:

```rust
  async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
      // use AxumRouter to mount your routes and return an AxumRouter
  }

```

If you want Loco to understand the metadata information about your routes (which can come in handy later), write your `routes()` function in each of your controllers in this way:


```rust
// this is what people usually do using Axum only
pub fn router() -> Router {
  Router::new()
        .route("/auth/register", post(create_user))
        .route("/auth/login", post(login_user))
}

// this is how it looks like using Loco (notice we use `Routes` and `add`)
pub fn routes() -> Routes {
  Routes::new()
      .add("/auth/register", post(create_user))
      .add("/auth/login", post(login_user))
}
```

### Verdict

* **A drop-in compatibility** - Loco uses Axum and keeps all of its building blocks intact so that you can just use your own existing Axum code with no efforts.
* **Route metadata for free** - one gap that Axum routers has is the ability to describe the currently configured routes, which can be used for listing or automatic OpenAPI schema generation. Loco has a small metadata layer to suppor this. If you use `Routes` you get it for free, while all of the different signatures remain compatible with Axum router.
