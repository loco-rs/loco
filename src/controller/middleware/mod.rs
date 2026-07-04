//! Base Middleware for Loco Application
//!
//! This module defines the various middleware components that Loco provides.
//! Each middleware is responsible for handling different aspects of request
//! processing, such as authentication, logging, CORS, compression, and error
//! handling. The middleware can be easily configured and applied to the
//! application's router.

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
#[cfg(feature = "embedded_assets")]
pub mod static_assets_embedded;
#[cfg(feature = "embedded_assets")]
pub use static_assets_embedded as static_assets;

#[cfg(not(feature = "embedded_assets"))]
pub mod static_assets;
pub mod timeout;

use axum::Router as AXRouter;
use serde::{Deserialize, Serialize};

use crate::{app::AppContext, environment::Environment, Result};

/// Trait representing the behavior of middleware components in the application.
/// When implementing a new middleware, make sure to go over this checklist:
/// * The name of the middleware should be an ID that is similar to the field
///   name in configuration (look at how `serde` calls it)
/// * Default value implementation should be paired with `serde` default
///   handlers and default serialization implementation. Which means deriving
///   `Default` will _not_ work. You can use `serde_json` and serialize a new
///   config from an empty value, which will cause `serde` default value
///   handlers to kick in.
/// * If you need completely blank values for configuration (for example for
///   testing), implement an `::empty() -> Self` call ad-hoc.
pub trait MiddlewareLayer {
    /// Returns the name of the middleware.
    /// This should match the name of the property in the containing
    /// `middleware` section in configuration (as named by `serde`)
    fn name(&self) -> &'static str;

    /// Returns whether the middleware is enabled or not.
    /// If the middleware is switchable, take this value from a configuration
    /// value
    fn is_enabled(&self) -> bool {
        true
    }

    /// Returns middleware config.
    ///
    /// # Errors
    /// when could not convert middleware to [`serde_json::Value`]
    fn config(&self) -> serde_json::Result<serde_json::Value>;

    /// Applies the middleware to the given Axum router and returns the modified
    /// router.
    ///
    /// # Errors
    ///
    /// If there is an issue when adding the middleware to the router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>>;
}

#[allow(clippy::unnecessary_lazy_evaluations)]
#[must_use]
pub fn default_middleware_stack(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
    // Shortened reference to middlewares
    let middlewares = &ctx.config.server.middlewares;

    vec![
        // Limit Payload middleware with a default if none
        Box::new(middlewares.limit_payload.clone().unwrap_or_default()),
        // CORS middleware with a default if none
        Box::new(middlewares.cors.clone().unwrap_or_else(|| cors::Cors {
            enable: false,
            ..Default::default()
        })),
        // Catch Panic middleware with a default if none
        Box::new(
            middlewares
                .catch_panic
                .clone()
                .unwrap_or_else(|| catch_panic::CatchPanic { enable: true }),
        ),
        // Etag middleware with a default if none
        Box::new(
            middlewares
                .etag
                .clone()
                .unwrap_or_else(|| etag::Etag { enable: true }),
        ),
        // Remote IP middleware with a default if none
        Box::new(
            middlewares
                .remote_ip
                .clone()
                .unwrap_or_else(|| remote_ip::RemoteIpMiddleware {
                    enable: false,
                    ..Default::default()
                }),
        ),
        // Compression middleware with a default if none
        Box::new(
            middlewares
                .compression
                .clone()
                .unwrap_or_else(|| compression::Compression { enable: false }),
        ),
        // Timeout Request middleware with a default if none
        Box::new(
            middlewares
                .timeout_request
                .clone()
                .unwrap_or_else(|| timeout::TimeOut {
                    enable: false,
                    ..Default::default()
                }),
        ),
        // Static Assets middleware with a default if none
        Box::new(middlewares.static_assets.clone().unwrap_or_else(|| {
            static_assets::StaticAssets {
                enable: false,
                ..Default::default()
            }
        })),
        // Secure Headers middleware with a default if none
        Box::new(middlewares.secure_headers.clone().unwrap_or_else(|| {
            secure_headers::SecureHeader {
                enable: false,
                ..Default::default()
            }
        })),
        // Logger middleware with default logger configuration
        Box::new(logger::new(
            &middlewares
                .logger
                .clone()
                .unwrap_or_else(|| logger::Config { enable: true }),
            &ctx.environment,
        )),
        // Request ID middleware with a default if none
        Box::new(
            middlewares
                .request_id
                .clone()
                .unwrap_or_else(|| request_id::RequestId { enable: true }),
        ),
        // Fallback middleware with a default if none
        Box::new(
            middlewares
                .fallback
                .clone()
                .unwrap_or_else(|| fallback::Fallback {
                    enable: ctx.environment != Environment::Production,
                    ..Default::default()
                }),
        ),
        // Powered by middleware with a default identifier
        Box::new(powered_by::new(ctx.config.server.ident.as_deref())),
    ]
}

/// Ergonomic, Rails-style edits to a middleware stack
/// (`Vec<Box<dyn MiddlewareLayer>>`), for use inside
/// [`crate::app::Hooks::middlewares`] when you want to tweak the default stack
/// (from [`default_middleware_stack`]) rather than rebuild it.
///
/// Middlewares are matched by [`MiddlewareLayer::name`]. Positions are relative
/// to the stack `Vec`. Note the application order: the stack is applied in `Vec`
/// order and each middleware wraps the router, so a middleware **later** in the
/// `Vec` runs **earlier** in request processing (tower is LIFO).
///
/// If no middleware matches `name`, the operation logs a warning; `insert_before`
/// / `insert_after` still append the new middleware (so it is not silently
/// dropped), while `replace` / `delete` leave the stack unchanged.
pub trait MiddlewareStackExt {
    /// Insert `middleware` immediately before the first middleware named `name`.
    fn insert_before(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self;
    /// Insert `middleware` immediately after the first middleware named `name`.
    fn insert_after(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self;
    /// Replace the first middleware named `name` with `middleware`.
    fn replace(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self;
    /// Remove the first middleware named `name`.
    fn delete(&mut self, name: &str) -> &mut Self;
}

impl MiddlewareStackExt for Vec<Box<dyn MiddlewareLayer>> {
    fn insert_before(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self {
        if let Some(idx) = self.iter().position(|m| m.name() == name) {
            self.insert(idx, middleware);
        } else {
            tracing::warn!(
                middleware = name,
                "insert_before: no middleware named `{name}` in the stack; appending instead"
            );
            self.push(middleware);
        }
        self
    }

    fn insert_after(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self {
        if let Some(idx) = self.iter().position(|m| m.name() == name) {
            self.insert(idx + 1, middleware);
        } else {
            tracing::warn!(
                middleware = name,
                "insert_after: no middleware named `{name}` in the stack; appending instead"
            );
            self.push(middleware);
        }
        self
    }

    fn replace(&mut self, name: &str, middleware: Box<dyn MiddlewareLayer>) -> &mut Self {
        if let Some(idx) = self.iter().position(|m| m.name() == name) {
            self[idx] = middleware;
        } else {
            tracing::warn!(
                middleware = name,
                "replace: no middleware named `{name}` in the stack; leaving stack unchanged"
            );
        }
        self
    }

    fn delete(&mut self, name: &str) -> &mut Self {
        if let Some(idx) = self.iter().position(|m| m.name() == name) {
            self.remove(idx);
        } else {
            tracing::warn!(
                middleware = name,
                "delete: no middleware named `{name}` in the stack; leaving stack unchanged"
            );
        }
        self
    }
}

/// Server middleware configuration structure.
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Compression for the response.
    pub compression: Option<compression::Compression>,

    /// Etag cache headers.
    pub etag: Option<etag::Etag>,

    /// Limit the payload request.
    pub limit_payload: Option<limit_payload::LimitPayload>,

    /// Logger and augmenting trace id with request data
    pub logger: Option<logger::Config>,

    /// Catch any code panic and log the error.
    pub catch_panic: Option<catch_panic::CatchPanic>,

    /// Setting a global timeout for requests
    pub timeout_request: Option<timeout::TimeOut>,

    /// CORS configuration
    pub cors: Option<cors::Cors>,

    /// Serving static assets
    #[serde(rename = "static")]
    pub static_assets: Option<static_assets::StaticAssets>,

    /// Sets a set of secure headers
    pub secure_headers: Option<secure_headers::SecureHeader>,

    /// Resolves the client's remote IP from a single, configured, trusted
    /// source (a header such as `X-Forwarded-For`, or the raw socket) when
    /// behind a proxy or load balancer.
    pub remote_ip: Option<remote_ip::RemoteIpMiddleware>,

    /// Configure fallback behavior when hitting a missing URL
    pub fallback: Option<fallback::Fallback>,

    /// Request ID
    pub request_id: Option<request_id::RequestId>,
}

#[cfg(test)]
mod stack_ext_tests {
    use super::{MiddlewareLayer, MiddlewareStackExt};

    struct DummyMw(&'static str);
    impl MiddlewareLayer for DummyMw {
        fn name(&self) -> &'static str {
            self.0
        }
        fn config(&self) -> serde_json::Result<serde_json::Value> {
            Ok(serde_json::Value::Null)
        }
        fn apply(
            &self,
            app: axum::Router<crate::app::AppContext>,
        ) -> crate::Result<axum::Router<crate::app::AppContext>> {
            Ok(app)
        }
    }

    fn stack(names: &[&'static str]) -> Vec<Box<dyn MiddlewareLayer>> {
        names
            .iter()
            .map(|n| Box::new(DummyMw(n)) as Box<dyn MiddlewareLayer>)
            .collect()
    }

    fn names(stack: &[Box<dyn MiddlewareLayer>]) -> Vec<&'static str> {
        stack.iter().map(|m| m.name()).collect()
    }

    #[test]
    fn insert_before_hit() {
        let mut s = stack(&["a", "b", "c"]);
        s.insert_before("b", Box::new(DummyMw("x")));
        assert_eq!(names(&s), vec!["a", "x", "b", "c"]);
    }

    #[test]
    fn insert_after_hit() {
        let mut s = stack(&["a", "b", "c"]);
        s.insert_after("b", Box::new(DummyMw("x")));
        assert_eq!(names(&s), vec!["a", "b", "x", "c"]);
    }

    #[test]
    fn insert_before_miss_appends() {
        let mut s = stack(&["a", "b", "c"]);
        s.insert_before("nope", Box::new(DummyMw("x")));
        assert_eq!(names(&s), vec!["a", "b", "c", "x"]);
    }

    #[test]
    fn insert_after_miss_appends() {
        let mut s = stack(&["a", "b", "c"]);
        s.insert_after("nope", Box::new(DummyMw("x")));
        assert_eq!(names(&s), vec!["a", "b", "c", "x"]);
    }

    #[test]
    fn replace_hit() {
        let mut s = stack(&["a", "b", "c"]);
        s.replace("b", Box::new(DummyMw("x")));
        assert_eq!(names(&s), vec!["a", "x", "c"]);
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn replace_miss_no_op() {
        let mut s = stack(&["a", "b", "c"]);
        s.replace("nope", Box::new(DummyMw("x")));
        assert_eq!(names(&s), vec!["a", "b", "c"]);
    }

    #[test]
    fn delete_hit() {
        let mut s = stack(&["a", "b", "c"]);
        s.delete("b");
        assert_eq!(names(&s), vec!["a", "c"]);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn delete_miss_no_op() {
        let mut s = stack(&["a", "b", "c"]);
        s.delete("nope");
        assert_eq!(names(&s), vec!["a", "b", "c"]);
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn chaining_returns_mut_self() {
        let mut s = stack(&["a", "b", "c"]);
        s.insert_before("a", Box::new(DummyMw("x")))
            .insert_after("c", Box::new(DummyMw("y")))
            .replace("b", Box::new(DummyMw("z")))
            .delete("a");
        assert_eq!(names(&s), vec!["x", "z", "c", "y"]);
    }
}
