//! This module defines the [`AppRoutes`] struct that is responsible for
//! configuring routes in an Axum application. It allows you to define route
//! prefixes, add routes, and configure middlewares for the application.

use std::time::Duration;

use axum::{http::Request, response::Response, Router as AXRouter};
use lazy_static::lazy_static;
use regex::Regex;
use tower_http::{catch_panic::CatchPanicLayer, trace::TraceLayer};
use tower_request_id::{RequestId, RequestIdLayer};

use super::{health, routes::Routes};
use crate::{app::AppContext, Result};

lazy_static! {
    static ref NORMALIZE_URL: Regex = Regex::new(r"/+").unwrap();
}

/// Represents the routes of the application.
#[derive(Clone)]
pub struct AppRoutes {
    prefix: Option<String>,
    routes: Vec<Routes>,
}

impl AppRoutes {
    /// Create a new instance with the default routes.
    #[must_use]
    pub fn with_default_routes() -> Self {
        Self::empty().add_route(health::routes())
    }

    /// Create an empty instance.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            prefix: None,
            routes: vec![],
        }
    }

    /// Get the prefix of the routes.
    #[must_use]
    pub fn get_prefix(&self) -> Option<&String> {
        self.prefix.as_ref()
    }

    /// Get the routes.
    #[must_use]
    pub fn get_routes(&self) -> &[Routes] {
        self.routes.as_ref()
    }

    /// Set a prefix for the routes. this prefix will be a prefix for all the
    /// routes.
    ///
    /// # Example
    ///
    /// In the following example you are adding api as a prefix for all routes
    ///
    /// ```rust
    /// use rustyrails::controller::AppRoutes;
    ///
    /// AppRoutes::with_default_routes().prefix("api");
    /// ```
    #[must_use]
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    /// Add a single route.
    #[must_use]
    pub fn add_route(mut self, route: Routes) -> Self {
        self.routes.push(route);
        self
    }

    /// Add multiple routes.
    #[must_use]
    pub fn add_routes(mut self, mounts: Vec<Routes>) -> Self {
        for mount in mounts {
            self.routes.push(mount);
        }
        self
    }

    /// Convert the routes to an Axum Router, and set a list of middlewares that
    /// configure in the [`config::Config`]
    ///
    /// # Errors
    /// Return an [`Result`] when could not convert the router setup to
    /// [`axum::Router`].
    pub fn to_router(&self, ctx: AppContext) -> Result<AXRouter> {
        let mut app = AXRouter::new();
        let base_url_prefix = self.get_prefix().map_or("/", |url| url.as_str());

        for router in self.get_routes() {
            let mut uri_parts = vec![base_url_prefix];

            if let Some(prefix) = router.prefix.as_ref() {
                uri_parts.push(prefix);
            }

            for controller in &router.handlers {
                let uri = format!("{}{}", uri_parts.join("/"), &controller.uri);

                let method = controller.method.clone();

                let uri = NORMALIZE_URL.replace_all(&uri, "/");

                tracing::info!("{}", &uri);

                app = app.route(&uri, method);
            }
        }

        if let Some(catch_panic) = &ctx.config.server.middlewares.catch_panic {
            if catch_panic.enable {
                // TODO:: handle better response
                app = app.layer(CatchPanicLayer::new());
            }
        }

        if let Some(limit) = &ctx.config.server.middlewares.limit_payload {
            if limit.enable {
                app = app.layer(axum::extract::DefaultBodyLimit::max(
                    byte_unit::Byte::from_str(&limit.body_limit)
                        .map_err(Box::from)?
                        .get_bytes() as usize,
                ));
                tracing::info!(
                    data = &limit.body_limit,
                    "[Middleware] Adding limit payload",
                );
            }
        }

        if let Some(logger) = &ctx.config.server.middlewares.logger {
            if logger.enable {
                app = app
                    .layer(
                        TraceLayer::new_for_http()
                            .make_span_with(|request: &Request<_>| {
                                let request_id = request
                                    .extensions()
                                    .get::<RequestId>()
                                    .map_or_else(|| "unknown".into(), ToString::to_string);
                                tracing::error_span!(
                                    "request",
                                    id = %request_id,
                                    method = %request.method(),
                                    uri = %request.uri(),
                                )
                            })
                            .on_response(
                                |response: &Response<_>,
                                 latency: Duration,
                                 _span: &tracing::Span| {
                                    tracing::info!(
                                        latency = format!("{latency:?}"),
                                        status = format!("{:?}", response.status()),
                                        "finished processing request",
                                    );
                                },
                            ),
                    )
                    .layer(RequestIdLayer);
                tracing::info!("[Middleware] Adding log trace id",);
            }
        }
        Ok(app.with_state(ctx))
    }
}
