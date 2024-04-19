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
and [`tower`](https://crates.io/crates/tower). They provide a way to add
[layer](https://docs.rs/tower/latest/tower/trait.Layer.html)
and [service](https://docs.rs/tower/latest/tower/trait.Service.html) as middleware to your routes and handlers.

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

use crate::models::{roles, sea_orm_active_enums::RolesName, users};

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
    // S is the inner service, in the case is the `/auth/register` handler
    inner: S,
}

/// Implement the Service trait for LogService
/// # Generics
/// * `S` - The inner service, in this case is the `/auth/register` handler
/// * `B` - The body type
impl<S, B> Service<Request<B>> for LogService<S>
    where
        S: Service<Request<B>, Response=Response<Body>, Error=Infallible> + Clone + Send + 'static, /* Inner Service must return Response<Body> and never error, which is most of the for handlers */
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

Usage: Add the middleware to the `auth` route.

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

Usage: Add the middleware to the `auth::register` handler.

```rust
// src/controllers/auth.rs
pub fn routes() -> Routes {
    Routes::new()
        .prefix("auth")
        .add("/register", post(register).layer(middlewares::log::LogLayer::new()))
}
```

Now when you make a request to the `auth::register` route, you will see the request method and path logged.

```shell
INFO  loco_rs::controllers::middleware::log: Request: POST "/auth/register"
```

In the first glance, this middleware is a bit complex. Let's break it down.

The `LogLayer` is a [`tower::Layer`](https://docs.rs/tower/latest/tower/trait.Layer.html) that wraps around the inner
service. The `LogService` is a
[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) that implements the `Service` trait for the
request.

### Generics Explanation

`S` is the inner service, in this case, it is the `/auth/register` handler. If we have a look about
the [`get`](https://docs.rs/axum/latest/axum/routing/method_routing/fn.get.html), [`post`](https://docs.rs/axum/latest/axum/routing/method_routing/fn.post.html), [`put`](https://docs.rs/axum/latest/axum/routing/method_routing/fn.put.html), [`delete`](https://docs.rs/axum/latest/axum/routing/method_routing/fn.delete.html)
functions which we use for handlers, they all return
a [`MethodRoute<S, Infallible>`(Which is a service)](https://docs.rs/axum/latest/axum/routing/method_routing/struct.MethodRouter.html).

Therefore, `S: Service<Request<B>, Response = Response<Body>, Error = Infallible>` means it takes in a `Request<B>`(
Request with a body) and returns a `Response<Body>`. The `Error` is `Infallible` which means the handler never errors.

`B` is the body type.

### Function Explanation

The `LogLayer::new` function is used to create a new instance of the `LogLayer`.

The `LogService::poll_ready` function is used to check if the service is ready to process a request. It can be used for
backpressure, for more information see the [`tower::Service`
documentation](https://docs.rs/tower/latest/tower/trait.Service.html)
and [Tokio tutorial](https://tokio.rs/blog/2021-05-14-inventing-the-service-trait#backpressure).

The `LogService::call` function is used to process the request. In this case, we are logging the request method and
path. Then we are calling the inner service with the request.

Note that in `LogService::call` we are cloning the inner service and `std::mem::replace` replacing it. This is because
services are permitted to panic if `LogService::call` is invoked without obtaining `Poll::Ready(Ok(()))`
from `LogService::poll_ready`.

Therefore, we should be careful when cloning services for example to move them into boxed
futures. Even though the original service is ready, the clone might not
be.[Tower Service Cloning Documentation](https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services)

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




