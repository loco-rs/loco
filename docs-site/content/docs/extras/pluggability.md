+++
title = "Pluggability"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

## Error levels and options

As a reminder, error levels and their logging can be controlled in your `development.yaml`:

### Logger

<!-- <snip id="configuration-logger" inject_from="code" template="yaml"> -->

```yaml
# Application logging configuration
logger:
  # Enable or disable logging.
  enable: true
  # Enable pretty backtrace (sets RUST_BACKTRACE=1)
  pretty_backtrace: true
  # Log level, options: trace, debug, info, warn or error.
  level: debug
  # Define the logging format. options: compact, pretty or json
  format: compact
  # By default the logger has filtering only logs that came from your code or logs that came from `loco` framework. to see all third party libraries
  # Uncomment the line below to override to see all third party libraries you can enable this config and override the logger filters.
  # override_filter: trace
```

<!-- </snip> -->

The most important knobs here are:

- `level` - your standard logging levels. Typically `debug` or `trace` in development. In production, choose what you are used to.
- `pretty_backtrace` - provides a clear, concise path to the line of code causing the error. Use `true` in development and turn it off in production. In cases where you are debugging things in production and need some extra hand, you can turn it on and then off when you're done.

### Controller logging

In `server.middlewares` you will find:

```yaml
server:
  middlewares:
    #
    # ...
    #
    # Generating a unique request ID and enhancing logging with additional information such as the start and completion of request processing, latency, status code, and other request details.
    logger:
      # Enable/Disable the middleware.
      enable: true
```

You should enable it to get detailed request errors and a useful `request-id` that can help collate multiple request-scoped errors.

### Database

You have the option of logging live SQL queries, in your `database` section:

```yaml
database:
  # When enabled, the sql query will be logged.
  enable_logging: false
```

### Operating around errors

You'll be mostly looking at your terminal for errors while developing your app, it can look something like this:

```bash
2024-02-xxx DEBUG http-request: tower_http::trace::on_request: started processing request http.method=GET http.uri=/notes http.version=HTTP/1.1 http.user_agent=curl/8.1.2 environment=development request_id=8622e624-9bda-49ce-9730-876f2a8a9a46
2024-02-xxx11T12:19:25.295954Z ERROR http-request: loco_rs::controller: controller_error error.msg=invalid type: string "foo", expected a sequence error.details=JSON(Error("invalid type: string \"foo\", expected a sequence", line: 0, column: 0)) error.chain="" http.method=GET http.uri=/notes http.version=HTTP/1.1 http.user_agent=curl/8.1.2 environment=development request_id=8622e624-9bda-49ce-9730-876f2a8a9a46
```

Usually you can expect the following from errors:

- `error.msg` a `to_string()` version of an error, for operators.
- `error.detail` a debug representation of an error, for developers.
- An error **type** e.g. `controller_error` as the primary message tailored for searching, rather than a verbal error message.
- Errors are logged as _tracing_ events and spans, so that you can build any infrastructure you want to provide custom tracing subscribers. Check out the [prometheus](https://github.com/loco-rs/loco-extras/blob/main/src/initializers/prometheus.rs) example in `loco-extras`.

Notes:

- An _error chain_ was experimented with, but provides little value in practice.
- Errors that an end user sees are a completely different thing. We strive to provide **minimal internal details** about an error for an end user when we know a user can't do anything about an error (e.g. "database offline error"), mostly it will be a generic "Internal Server Error" on purpose -- for security reasons.

### Producing errors

When you build controllers, you write your handlers to return `Result<impl IntoResponse>`. The `Result` here is a Loco `Result`, which means it also associates a Loco `Error` type.

If you reach out for the Loco `Error` type you can use any of the following as a response:

```rust
Err(Error::string("some custom message"));
Err(Error::msg(other_error)); // turns other_error to its string representation
Err(Error::wrap(other_error));
Err(Error::Unauthorized("some message"))

// or through controller helpers:
unauthorized("some message") // create a full response object, calling Err on a created error
```

## Initializers

Initializers are a way to encapsulate a piece of infrastructure "wiring" that you need to do in your app. You put initializers in `src/initializers/`.

### Writing initializers

Currently, an initializer is anything that implements the `Initializer` trait:

<!-- <snip id="initializers-trait" inject_from="code" template="rust"> -->

```rust
pub trait Initializer: Sync + Send {
    /// The initializer name or identifier
    fn name(&self) -> String;

    /// Occurs after the app's `before_run`.
    /// Use this to for one-time initializations, load caches, perform web
    /// hooks, etc.
    async fn before_run(&self, _app_context: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Occurs after the app's `after_routes`.
    /// Use this to compose additional functionality and wire it into an Axum
    /// Router
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router)
    }

    /// Perform health checks for this initializer.
    /// This method is called during the doctor command to validate the initializer's configuration.
    /// Return `None` if no check is needed, or `Some(Check)` if a check should be performed.
    async fn check(&self, _app_context: &AppContext) -> Result<Option<crate::doctor::Check>> {
        Ok(None)
    }
}
```

<!-- </snip> -->

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

<!-- <snip id="app-initializers" inject_from="code" template="rust"> -->

```rust
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        let initializers: Vec<Box<dyn Initializer>> = vec![
            Box::new(initializers::axum_session::AxumSessionInitializer),
            Box::new(initializers::view_engine::ViewEngineInitializer),
            Box::new(initializers::hello_view_engine::HelloViewEngineInitializer),
        ];

        Ok(initializers)
    }
```

<!-- </snip> -->

Loco will now run your initializer stack in the correct places during the app boot process.

### Initializer Health Checks

Initializers can now provide their own health checks by implementing the `check` method. This allows each initializer to validate its configuration and test its connections during the `cargo loco doctor` command.

#### Implementing Health Checks

To add health checks to your initializer, implement the `check` method:

```rust
use async_trait::async_trait;
use loco_rs::app::{AppContext, Initializer};
use loco_rs::doctor::{Check, CheckStatus};

struct MyCustomInitializer;

#[async_trait]
impl Initializer for MyCustomInitializer {
    fn name(&self) -> String {
        "my_custom_initializer".to_string()
    }

    async fn check(&self, app_context: &AppContext) -> loco_rs::Result<Option<Check>> {
        // Check if your configuration exists
        let config = app_context.config.initializers.as_ref()
            .and_then(|init| init.get("my_custom_initializer"))
            .ok_or_else(|| loco_rs::Error::Message("Configuration not found".to_string()))?;

        // Perform your health check
        match self.test_connection(config).await {
            Ok(()) => Ok(Some(Check {
                status: CheckStatus::Ok,
                message: "My custom service: success".to_string(),
                description: None,
            })),
            Err(err) => Ok(Some(Check {
                status: CheckStatus::NotOk,
                message: "My custom service: failed".to_string(),
                description: Some(err.to_string()),
            })),
        }
    }
}
```

#### Health Check Return Values

The `check` method returns `Result<Option<Check>>`:

- **`Ok(None)`**: No health check needed (default behavior)
- **`Ok(Some(Check))`**: Health check result to be displayed

#### Check Status Types

```rust
pub enum CheckStatus {
    Ok,           // ✅ Component is healthy
    NotOk,        // ❌ Component has issues
    NotConfigure, // ⚠️ Component not configured (may be intentional)
}
```

#### Running Initializer Health Checks

Health checks are automatically run when you execute:

```sh
cargo loco doctor
```

The output will include your initializer checks:

```
✅ Database connection: success
✅ Initializer my_custom_initializer: My custom service: success
❌ Initializer failing_service: Service connection: failed
   connection timeout after 30 seconds
```

#### Optional Health Checks

Health checks are completely optional. If your initializer doesn't implement the `check` method, it will use the default implementation that returns `Ok(None)`, meaning no health check will be performed.

This makes the feature backward-compatible and allows initializers to opt-in to health checking when needed.

### What other things you can do?

Right now initializers contain two integration points:

- `before_run` - happens before running the app -- this is a pure "initialization" type of a hook. You can send web hooks, metric points, do cleanups, pre-flight checks, etc.
- `after_routes` - happens after routes have been added. You have access to the Axum router and its powerful layering integration points, this is where you will spend most of your time.

### Compared to Rails initializers

Rails initializers, are regular scripts that run once -- for initialization and have access to everything. They get their power from being able to access a "live" Rails app, modify it as a global instance.

In Loco, accessing a global instance and mutating it is not possible in Rust (for a good reason!), and so we offer two integration points which are explicit and safe:

1. Pure initialization (without any influence on a configured app)
2. Integration with a running app (via Axum router)

Rails initializers need _ordering_ and _modification_. Meaning, a user should be certain that they run in a specific order (or re-order them), and a user is able to remove initializers that other people set before them.

In Loco, we circumvent this complexity by making the user _provide a full vec_ of initializers. Vecs are ordered, and there are no implicit initializers.

### The global logger initializer

Some developers would like to customize their logging stack. In Loco this involves setting up tracing and tracing subscribers.

Because at the moment tracing does not allow for re-initialization, or modification of an in-flight tracing stack, you _only get one chance to initialize and registr a global tracing stack_.

This is why we added a new _App level hook_, called `init_logger`, which you can use to provide your own logging stack initialization.

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

## Middlewares

`Loco` is a framework that is built on top of [`axum`](https://crates.io/crates/axum)
and [`tower`](https://crates.io/crates/tower). They provide a way to
add [layers](https://docs.rs/tower/latest/tower/trait.Layer.html)
and [services](https://docs.rs/tower/latest/tower/trait.Service.html) as middleware to your routes and handlers.

Middleware is a way to add pre- and post-processing to your requests. This can be used for logging, authentication, rate
limiting, route-specific processing, and more.

### Source Code

`Loco`'s implementation of route middleware/layer is similar
to `axum`'s [`Router::layer`](https://github.com/tokio-rs/axum/blob/main/axum/src/routing/mod.rs#L275). You can
find the source code for middleware in
the [`src/controllers/routes`](https://github.com/loco-rs/loco/blob/master/src/controller/routes.rs) directory.
This `layer` function will attach the
middleware layer to each handler of the route.

```rust
// src/controller/routes.rs
use axum::{extract::Request, response::IntoResponse, routing::Route};
use tower::{Layer, Service};

impl Routes {
    pub fn layer<L>(self, layer: L) -> Self
        where
            L: Layer<Route> + Clone + Send + 'static,
            L::Service: Service<Request> + Clone + Send + 'static,
            <L::Service as Service<Request>>::Response: IntoResponse + 'static,
            <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
            <L::Service as Service<Request>>::Future: Send + 'static,
    {
        Self {
            prefix: self.prefix,
            handlers: self
                .handlers
                .iter()
                .map(|handler| Handler {
                    uri: handler.uri.clone(),
                    actions: handler.actions.clone(),
                    method: handler.method.clone().layer(layer.clone()),
                })
                .collect(),
        }
    }
}
```

### Basic Middleware

In this example, we will create a basic middleware that will log the request method and path.

```rust
// src/controllers/middleware/log.rs
use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::{FromRequestParts, Request},
    response::Response,
};
use futures_util::future::BoxFuture;
use loco_rs::prelude::{auth::JWTWithUser, *};
use tower::{Layer, Service};

use crate::models::{users};

#[derive(Clone)]
pub struct LogLayer;

impl LogLayer {
    pub fn new() -> Self {
        Self {}
    }
}

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
        }
    }
}

#[derive(Clone)]
pub struct LogService<S> {
    // S is the inner service, in the case, it is the `/auth/register` handler
    inner: S,
}

/// Implement the Service trait for LogService
/// # Generics
/// * `S` - The inner service, in this case is the `/auth/register` handler
/// * `B` - The body type
impl<S, B> Service<Request<B>> for LogService<S>
    where
        S: Service<Request<B>, Response=Response<Body>, Error=Infallible> + Clone + Send + 'static, /* Inner Service must return Response<Body> and never error, which is typical for handlers */
        S::Future: Send + 'static,
        B: Send + 'static,
{
    // Response type is the same as the inner service / handler
    type Response = S::Response;
    // Error type is the same as the inner service / handler
    type Error = S::Error;
    // Future type is the same as the inner service / handler
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    // poll_ready is used to check if the service is ready to process a request
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Our middleware doesn't care about backpressure, so it's ready as long
        // as the inner service is ready.
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let clone = self.inner.clone();
        // take the service that was ready
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let (mut parts, body) = req.into_parts();
            tracing::info!("Request: {:?} {:?}", parts.method, parts.uri.path());
            let req = Request::from_parts(parts, body);
            inner.call(req).await
        })
    }
}
```

At the first glance, this middleware is a bit overwhelming. Let's break it down.

The `LogLayer` is a [`tower::Layer`](https://docs.rs/tower/latest/tower/trait.Layer.html) that wraps around the inner
service.

The `LogService` is a [`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) that implements
the `Service` trait for the request.

### Generics Explanation

**`Layer`**

In the `Layer` trait, `S` represents the inner service, which in this case is the `/auth/register` handler. The `layer`
function takes this inner service and returns a new service that wraps around it.

**`Service`**

`S` is the inner service, in this case, it is the `/auth/register` handler. If we have a look about
the [`get`](https://docs.rs/axum/latest/axum/routing/method_routing/fn.get.html), [`post`](https://docs.rs/axum/latest/axum/routing/method_routing/fn.post.html), [`put`](https://docs.rs/axum/latest/axum/routing/method_routing/fn.put.html), [`delete`](https://docs.rs/axum/latest/axum/routing/method_routing/fn.delete.html)
functions which we use for handlers, they all return
a [`MethodRoute<S, Infallible>`(Which is a service)](https://docs.rs/axum/latest/axum/routing/method_routing/struct.MethodRouter.html).

Therefore, `S: Service<Request<B>, Response = Response<Body>, Error = Infallible>` means it takes in a `Request<B>`(
Request with a body) and returns a `Response<Body>`. The `Error` is `Infallible` which means the handler never errors.

`S::Future: Send + 'static` means the future of the inner service must implement `Send` trait and `'static`.

`type Response = S::Response` means the response type of the middleware is the same as the inner service.

`type Error = S::Error` means the error type of the middleware is the same as the inner service.

`type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>` means the future type of the middleware is the
same as the inner service.

`B: Send + 'static` means the request body type must implement the `Send` trait and `'static`.

### Function Explanation

**`LogLayer`**

The `LogLayer::new` function is used to create a new instance of the `LogLayer`.

**`LogService`**

The `LogService::poll_ready` function is used to check if the service is ready to process a request. It can be used for
backpressure, for more information see
the [`tower::Service` documentation](https://docs.rs/tower/latest/tower/trait.Service.html)
and [Tokio tutorial](https://tokio.rs/blog/2021-05-14-inventing-the-service-trait#backpressure).

The `LogService::call` function is used to process the request. In this case, we are logging the request method and
path. Then we are calling the inner service with the request.

**Importance of `poll_ready`:**

In the Tower framework, before a service can be used to handle a request, it must be
checked for readiness
using the
`poll_ready` method. This method returns `Poll::Ready(Ok(()))` when the service is ready to process a request. If a
service is not ready, it may return `Poll::Pending`, indicating that the caller should wait before sending a request.
This mechanism ensures that the service has the necessary resources or state to process the request efficiently and
correctly.

**Cloning and Readiness**

When cloning a service, particularly to move it into a boxed future or similar context, it's crucial to understand that
the clone does not inherit the readiness state of the original service. Each clone of a service maintains its own state.
This means that even if the original service was ready `(Poll::Ready(Ok(())))`, the cloned service might not be in the
same state immediately after cloning. This can lead to issues where a cloned service is used before it is ready,
potentially causing panics or other failures.

**Correct approach to cloning services using `std::mem::replace`**
To handle cloning correctly, it's recommended to use `std::mem::replace` to swap the ready service with its clone in a
controlled manner. This approach ensures that the service being used to handle the request is the one that has been
verified as ready. Here's how it works:

- Clone the service: First, create a clone of the service. This clone will eventually replace the original service in
  the service handler.
- Replace the original with the clone: Use `std::mem::replace` to swap the original service with the clone. This
  operation ensures that the service handler continues to hold a service instance.
- Use the original service to handle the request: Since the original service was already checked for readiness (via
  `poll_ready`), it's safe to use it to handle the incoming request. The clone, now in the handler, will be the one
  checked for readiness next time.

This method ensures that each service instance used to handle requests is always the one that has been explicitly
checked for readiness, thus maintaining the integrity and reliability of the service handling process.

Here is a simplified example to illustrate this pattern:

```rust
// Wrong
fn call(&mut self, req: Request<B>) -> Self::Future {
    let mut inner = self.inner.clone();
    Box::pin(async move {
        /* ... */
        inner.call(req).await
    })
}

// Correct
fn call(&mut self, req: Request<B>) -> Self::Future {
    let clone = self.inner.clone();
    // take the service that was ready
    let mut inner = std::mem::replace(&mut self.inner, clone);
    Box::pin(async move {
        /* ... */
        inner.call(req).await
    })
}
```

In this example, `inner` is the service that was ready, and after handling the request, `self.inner` now holds the
clone, which will be checked for readiness in the next cycle. This careful management of service readiness and cloning
is essential for maintaining robust and error-free service operations in asynchronous Rust applications using Tower.

[Tower Service Cloning Documentation](https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services)

### Adding Middleware to Handler

Add the middleware to the `auth::register` handler.

```rust
// src/controllers/auth.rs
pub fn routes() -> Routes {
    Routes::new()
        .prefix("auth")
        .add("/register", post(register).layer(middlewares::log::LogLayer::new()))
}
```

Now when you make a request to the `auth::register` handler, you will see the request method and path logged.

```shell
2024-XX-XXTXX:XX:XX.XXXXXZ  INFO http-request: xx::controllers::middleware::log Request: POST "/auth/register" http.method=POST http.uri=/auth/register http.version=HTTP/1.1  environment=development request_id=xxxxx
```

## Adding Middleware to Route

Add the middleware to the `auth` route.

```rust
// src/main.rs
pub struct App;

#[async_trait]
impl Hooks for App {
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(
                controllers::auth::routes()
                    .layer(middlewares::log::LogLayer::new()),
            )
    }
}
```

Now when you make a request to any handler in the `auth` route, you will see the request method and path logged.

```shell
2024-XX-XXTXX:XX:XX.XXXXXZ  INFO http-request: xx::controllers::middleware::log Request: POST "/auth/register" http.method=POST http.uri=/auth/register http.version=HTTP/1.1  environment=development request_id=xxxxx
```

### Advanced Middleware (With AppContext)

There will be times when you need to access the `AppContext` in your middleware. For example, you might want to access
the database connection to perform some authorization checks. To do this, you can add the `AppContext` to
the `Layer` and `Service`.

Here we will create a middleware that checks the JWT token and gets the user from the database then prints the user's
name

```rust
// src/controllers/middleware/log.rs
use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::{FromRequestParts, Request},
    response::Response,
};
use futures_util::future::BoxFuture;
use loco_rs::prelude::{auth::JWTWithUser, *};
use tower::{Layer, Service};

use crate::models::{users};

#[derive(Clone)]
pub struct LogLayer {
    state: AppContext,
}

impl LogLayer {
    pub fn new(state: AppContext) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct LogService<S> {
    inner: S,
    state: AppContext,
}

impl<S, B> Service<Request<B>> for LogService<S>
    where
        S: Service<Request<B>, Response=Response<Body>, Error=Infallible> + Clone + Send + 'static, /* Inner Service must return Response<Body> and never error */
        S::Future: Send + 'static,
        B: Send + 'static,
{
    // Response type is the same as the inner service / handler
    type Response = S::Response;
    // Error type is the same as the inner service / handler
    type Error = S::Error;
    // Future type is the same as the inner service / handler
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let state = self.state.clone();
        let clone = self.inner.clone();
        // take the service that was ready
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            // Example of extracting JWT token from the request
            let (mut parts, body) = req.into_parts();
            let auth = JWTWithUser::<users::Model>::from_request_parts(&mut parts, &state).await;

            match auth {
                Ok(auth) => {
                    // Example of getting user from the database
                    let user = users::Model::find_by_email(&state.db, &auth.user.email).await.unwrap();
                    tracing::info!("User: {}", user.name);
                    let req = Request::from_parts(parts, body);
                    inner.call(req).await
                }
                Err(_) => {
                    // Handle error, e.g., return an unauthorized response
                    Ok(Response::builder()
                        .status(401)
                        .body(Body::empty())
                        .unwrap()
                        .into_response())
                }
            }
        })
    }
}
```

In this example, we have added the `AppContext` to the `LogLayer` and `LogService`. We are using the `AppContext` to get
the database connection and the JWT token for pre-processing.

### Adding Middleware to Route (advanced)

Add the middleware to the `notes` route.

```rust
// src/app.rs
pub struct App;

#[async_trait]
impl Hooks for App {
    fn routes(ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::notes::routes().layer(middlewares::log::LogLayer::new(ctx)))
    }
}
```

Now when you make a request to any handler in the `notes` route, you will see the user's name logged.

```shell
2024-XX-XXTXX:XX:XX.XXXXXZ  INFO http-request: xx::controllers::middleware::log User: John Doe  environment=development request_id=xxxxx
```

### Adding Middleware to Handler (advanced)

In order to add the middleware to the handler, you need to add the `AppContext` to the `routes` function
in `src/app.rs`.

```rust
// src/app.rs
pub struct App;

#[async_trait]
impl Hooks for App {
    fn routes(ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(
                controllers::notes::routes(ctx)
            )
    }
}
```

Then add the middleware to the `notes::create` handler.

```rust
// src/controllers/notes.rs
pub fn routes(ctx: &AppContext) -> Routes {
    Routes::new()
        .prefix("notes")
        .add("/create", post(create).layer(middlewares::log::LogLayer::new(ctx)))
}
```

Now when you make a request to the `notes::create` handler, you will see the user's name logged.

```shell
2024-XX-XXTXX:XX:XX.XXXXXZ  INFO http-request: xx::controllers::middleware::log User: John Doe  environment=development request_id=xxxxx
```

## Application SharedStore

Loco provides a flexible mechanism called `SharedStore` within the `AppContext` to store and share arbitrary custom data or services across your application. This feature allows you to inject your own types into the application context without modifying Loco's core structures, enhancing pluggability and customization.

`AppContext.shared_store` is a type-safe, thread-safe heterogeneous storage. You can store any type that implements `'static + Send + Sync`.

### Why Use SharedStore?

- **Sharing Custom Services:** Inject your own service clients (e.g., a custom API client) and access them from controllers or background workers.
- **Storing Configuration:** Keep application-specific configuration objects accessible globally.
- **Shared State:** Manage state needed by different parts of your application.

### How to Use SharedStore

You typically insert your custom data into the `shared_store` during application startup (e.g., in `src/app.rs`) and then retrieve it within your controllers or other components.

**1. Define Your Data Structures:**

Create the structs for the data or services you want to share. Note whether they implement `Clone`.

```rust
// In src/app.rs or a dedicated module (e.g., src/services.rs)

// This service can be cloned
#[derive(Clone, Debug)]
pub struct MyClonableService {
    pub api_key: String,
}

// This service cannot (or should not) be cloned
#[derive(Debug)]
pub struct MyNonClonableService {
    pub api_key: String,
}
```

**2. Insert into SharedStore (in `src/app.rs`):**

A good place to insert your shared data is the `after_context` hook in your `App`'s `Hooks` implementation.

```rust
// In src/app.rs

use crate::MyClonableService; // Import your structs
use crate::MyNonClonableService;

pub struct App;
#[async_trait]
impl Hooks for App {
    // ... other Hooks methods (app_name, boot, etc.) ...

    async fn after_context(mut ctx: AppContext) -> Result<AppContext> {
        // Create instances of your services/data
        let clonable_service = MyClonableService {
            api_key: "key-cloned-12345".to_string(),
        };
        let non_clonable_service = MyNonClonableService {
            api_key: "key-ref-67890".to_string(),
        };

        // Insert them into the shared store
        ctx.shared_store.insert(clonable_service);
        ctx.shared_store.insert(non_clonable_service);

        Ok(ctx)
    }

    // ... rest of Hooks implementation ...
}
```

**3. Retrieve from SharedStore (in Controllers):**

You have two main ways to retrieve data in your controllers:

- **Using the `SharedStore(var)` Extractor (for `Clone`-able types):**
  This is the most convenient way if your type implements `Clone`. The extractor retrieves and _clones_ the data for you.

  ```rust
  // In src/controllers/some_controller.rs
  use loco_rs::prelude::*;
  use crate::app::MyClonableService; // Or wherever it's defined

  #[axum::debug_handler]
  pub async fn index(
      // Extracts and clones MyClonableService into `service`
      SharedStore(service): SharedStore<MyClonableService>,
  ) -> impl IntoResponse {
      tracing::info!("Using Cloned Service API Key: {}", service.api_key);
      format::empty()
  }
  ```

- **Using `ctx.shared_store.get_ref()` (for Non-`Clone`-able types or avoiding clones):**
  Use this method when your type doesn't implement `Clone` or when you want to avoid the performance cost of cloning. It gives you a reference (`RefGuard<T>`) to the data.

  ```rust
  // In src/controllers/some_controller.rs
  use loco_rs::prelude::*;
  use crate::app::MyNonClonableService; // Or wherever it's defined

  #[axum::debug_handler]
  pub async fn index(
      State(ctx): State<AppContext>, // Need the AppContext state
  ) -> Result<impl IntoResponse> {
      // Get a reference to the non-clonable service
      let service_ref = ctx.shared_store.get_ref::<MyNonClonableService>()
          .ok_or_else(|| {
              tracing::error!("MyNonClonableService not found in shared store");
              Error::InternalServerError // Or a more specific error
          })?;

      // Access fields via the reference guard
      tracing::info!("Using Non-Cloned Service API Key: {}", service_ref.api_key);
      format::empty()
  }
  ```

**Summary:**

- Use `SharedStore` in `AppContext` to share custom services or data.
- Insert data during app setup (e.g., `after_context` in `src/app.rs`).
- Use the `SharedStore(var)` extractor for convenient access to `Clone`-able types (clones the data).
- Use `ctx.shared_store.get_ref::<T>()` to get a reference to non-`Clone`-able types or to avoid cloning for performance reasons.
