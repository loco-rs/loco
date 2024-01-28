+++
title = "Initializers"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 19
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

Initializers are a way to encapsulate a piece of infrastructure "wiring" that you need to do in your app. You put initializers in `src/initializers/`.

### Writing initializers

Currently, an initializer is anything that implements the `Initializer` trait:

```rust
// implement this and place your code in `src/initializers/`
pub trait Initializer {
    fn name(&self) -> String;
    async fn before_run(&self, _app_context: &AppContext) -> Result<()>; 
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter>;
}
```

### Example: Integrating Axum Session

You might want to add sessions to your app using `axum-session`. Also, you might want to share that piece of functionality between your own projects, or grab that piece of code from someone else.

You can achieve this reuse easily, if you code the integration as an _initializer_:

```rust
// place this in `src/initializers/axum_session.rs`
#[async_trait]
impl Initializer for AxumSessionInitializer {
    fn name(&self) -> String {
        "axum-session".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let session_config =
            axum_session::SessionConfig::default().with_table_name("sessions_table");
        let session_store =
            axum_session::SessionStore::<axum_session::SessionNullPool>::new(None, session_config)
                .await
                .unwrap();
        let router = router.layer(axum_session::SessionLayer::new(session_store));
        Ok(router)
    }
}
```

And now your app structure looks like this:

```
src/
 bin/
 controllers/
    :
    :
 initializers/       <--- a new folder
   mod.rs            <--- a new module
   axum_session.rs   <--- your new initializer
    :
    :
  app.rs   <--- register initializers here
```


### Using initializers

After you've implemented your own initializer, you should implement the `initializers(..)` hook in your `src/app.rs` and provide a Vec of your initializers:


```rust
// in src/app.rs
impl Hooks for App {
    // return a vec of boxed initializers (`Box<dyn Initializer>`)
    // lets Loco know what to use.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![Box::new(
            initializers::axum_session::AxumSessionInitializer,
        )])
    }
}
```

Loco will now run your initializer stack in the correct places during the app boot process.

### What other things you can do?

Right now initializers contain two integration points:

* `before_run` - happens before running the app -- this is a pure "initialization" type of a hook. You can send web hooks, metric points, do cleanups, pre-flight checks, etc.
* `after_routes` - happens after routes have been added. You have access to the Axum router and its powerful layering integration points, this is where you will spend most of your time.

### Compared to Rails initializers

Rails initializers, are regular scripts that run once -- for initialization and have access to everything. They get their power from being able to access a "live" Rails app, modify it as a global instance. 

In Loco, accessing a global instance and mutating it is not possible in Rust (for a good reason!), and so we offer two integration points which are explicit and safe:

1. Pure initialization (without any influence on a configured app)
2. Integration with a running app (via Axum router)

Rails initializers need _ordering_ and _modification_. Meaning, a user should be certain that they run in a specific order (or re-order them), and a user is able to remove initializers that other people set before them.

In Loco, we circumvent this complexity by making the user _provide a full vec_ of initializers. Vecs are ordered, and there are no implicit initializers. 

### The global logger initializer

Some developers would like to customize their logging stack. In Loco this involves setting up tracing and tracing subscribers.

Because at the moment tracing does not allow for re-initialization, or modification of an in-flight tracing stack, you *only get one chance to initialize and registr a global tracing stack*.

This is why we added a new *App level hook*, called `init_logger`, which you can use to provide your own logging stack initialization.

```rust
// in src/app.rs
impl Hooks for App {
    // return `Ok(true)` if you took over initializing logger
    // otherwise, return `Ok(false)` to use the Loco logging stack.
    fn init_logger(_config: &config::Config, _env: &Environment) -> Result<bool> {
        Ok(false)
    }
}
```

After you've set up your own logger, return `Ok(true)` to signal that you took over initialization.
