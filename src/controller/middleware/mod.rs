//! Base Middleware for Loco Application
//!
//! This module defines the various middleware components that Loco provides.
//! Each middleware is responsible for handling different aspects of request processing, such as
//! authentication, logging, CORS, compression, and error handling. The middleware can be easily
//! configured and applied to the application's router.

#[cfg(all(feature = "auth_jwt", feature = "with-db"))]
pub mod auth;
pub mod catch_panic;
pub mod compression;
pub mod cors;
pub mod etag;
pub mod fallback;
pub mod format;
pub mod limit_payload;
pub mod logger;
pub mod powered_by;
pub mod remote_ip;
pub mod request_id;
pub mod secure_headers;
pub mod static_assets;
pub mod timeout;
use crate::{app::AppContext, Result};
use axum::Router as AXRouter;
use serde::{Deserialize, Serialize};

/// Trait representing the behavior of middleware components in the application.
pub trait MiddlewareLayer {
    /// Returns the name of the middleware.
    fn name(&self) -> &'static str;

    /// Returns whether the middleware is enabled or not.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Applies the middleware to the given Axum router and returns the modified router.
    ///
    /// # Errors
    ///
    /// If there is an issue when adding the middleware to the router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>>;
}

/// Constructs a default stack of middleware for the Axum application based on the provided context.
///
/// This function initializes and returns a vector of middleware components that are commonly used
/// in the application. Each middleware is created using its respective `new` function and
#[must_use]
pub fn default_middleware_stack(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
    vec![
        Box::new(ctx.config.server.middlewares.limit_payload.clone()),
        Box::new(ctx.config.server.middlewares.cors.clone()),
        Box::new(ctx.config.server.middlewares.catch_panic.clone()),
        Box::new(ctx.config.server.middlewares.etag.clone()),
        Box::new(ctx.config.server.middlewares.remote_ip.clone()),
        Box::new(ctx.config.server.middlewares.compression.clone()),
        Box::new(ctx.config.server.middlewares.timeout_request.clone()),
        Box::new(ctx.config.server.middlewares.static_assets.clone()),
        Box::new(ctx.config.server.middlewares.secure_headers.clone()),
        Box::new(logger::new(
            &ctx.config.server.middlewares.logger,
            &ctx.environment,
        )),
        Box::new(ctx.config.server.middlewares.request_id.clone()),
        Box::new(ctx.config.server.middlewares.fallback.clone()),
        Box::new(powered_by::new(ctx.config.server.ident.as_deref())),
    ]
}

/// Server middleware configuration structure.
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Middleware that enable compression for the response.
    #[serde(default)]
    pub compression: compression::Compression,
    /// Middleware that enable etag cache headers.
    #[serde(default)]
    pub etag: etag::Etag,
    /// Middleware that limit the payload request.
    #[serde(default)]
    pub limit_payload: limit_payload::Config,
    /// Middleware that improve the tracing logger and adding trace id for each
    /// request.
    #[serde(default)]
    pub logger: logger::Config,
    /// catch any code panic and log the error.
    #[serde(default)]
    pub catch_panic: catch_panic::CatchPanic,
    /// Setting a global timeout for the requests
    #[serde(default)]
    pub timeout_request: timeout::TimeOut,
    /// Setting cors configuration
    #[serde(default)]
    pub cors: cors::Cors,
    /// Serving static assets
    #[serde(rename = "static")]
    #[serde(default)]
    pub static_assets: static_assets::StaticAssets,
    /// Sets a set of secure headers
    #[serde(default)]
    pub secure_headers: secure_headers::SecureHeader,
    /// Calculates a remote IP based on `X-Forwarded-For` when behind a proxy
    #[serde(default)]
    pub remote_ip: remote_ip::RemoteIpMiddleware,
    /// Configure fallback behavior when hitting a missing URL
    #[serde(default)]
    pub fallback: fallback::Fallback,
    #[serde(default)]
    pub request_id: request_id::RequestId,
}
