//! Base Middleware for Loco Application
//!
//! This module defines the various middleware components that Loco provides.
//! Each middleware is responsible for handling different aspects of request
//! processing, such as authentication, logging, CORS, compression, and error
//! handling. The middleware can be easily configured and applied to the
//! application's router.

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
use axum::Router as AXRouter;
use serde::{Deserialize, Serialize};

use crate::{app::AppContext, Result};

/// Trait representing the behavior of middleware components in the application.
pub trait MiddlewareLayer {
    /// Returns the name of the middleware.
    fn name(&self) -> &'static str;

    /// Returns whether the middleware is enabled or not.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Applies the middleware to the given Axum router and returns the modified
    /// router.
    ///
    /// # Errors
    ///
    /// If there is an issue when adding the middleware to the router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>>;
}

/// Constructs a default stack of middleware for the Axum application based on
/// the provided context.
///
/// This function initializes and returns a vector of middleware components that
/// are commonly used in the application. Each middleware is created using its
/// respective `new` function and
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
    /// Compression for the response.
    pub compression: compression::Compression,
    /// Etag cache headers.
    pub etag: etag::Etag,
    /// Limit the payload request.
    pub limit_payload: limit_payload::LimitPayload,
    /// Logger and augmenting trace id with request data
    pub logger: logger::Config,
    /// Catch any code panic and log the error.
    pub catch_panic: catch_panic::CatchPanic,
    /// Setting a global timeout for requests
    pub timeout_request: timeout::TimeOut,
    /// CORS configuration
    pub cors: cors::Cors,
    /// Serving static assets
    #[serde(rename = "static")]
    pub static_assets: static_assets::StaticAssets,
    /// Sets a set of secure headers
    pub secure_headers: secure_headers::SecureHeader,
    /// Calculates a remote IP based on `X-Forwarded-For` when behind a proxy
    pub remote_ip: remote_ip::RemoteIpMiddleware,
    /// Configure fallback behavior when hitting a missing URL
    pub fallback: fallback::Fallback,
    /// Request ID
    pub request_id: request_id::RequestId,
}
