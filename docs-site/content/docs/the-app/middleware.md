+++
title = "Middleware (Layer)"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 31
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair = []
+++

`Loco` is a framework that is built on top of [`axum`](https://crates.io/crates/axum)
and [`tower`](https://crates.io/crates/tower). They provide a way to
add [layers](https://docs.rs/tower/latest/tower/trait.Layer.html)
and [services](https://docs.rs/tower/latest/tower/trait.Service.html) as middleware to your routes and handlers.

Middleware is a way to add pre- and post-processing to your requests. This can be used for logging, authentication, rate
limiting, route specific processing, and more.

# Quick Start

## Basic Middleware

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
poll_ready method. This method returns `Poll::Ready(Ok(()))` when the service is ready to process a request. If a
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
verified as ready. Here’s how it works:

- Clone the service: First, create a clone of the service. This clone will eventually replace the original service in
  the service handler.
- Replace the original with the clone: Use `std::mem::replace` to swap the original service with the clone. This
  operation ensures that the service handler continues to hold a service instance.
- Use the original service to handle the request: Since the original service was already checked for readiness (via
  poll_ready), it's safe to use it to handle the incoming request. The clone, now in the handler, will be the one
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

## Basic Example Usage - Adding Middleware to Handler

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

## Basic Example Usage - Adding Middleware to Route

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

## Advanced Middleware (With AppContext)

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

## Advanced Example Usage - Adding Middleware to Route

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

## Advanced Example Usage - Adding Middleware to Handler

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




