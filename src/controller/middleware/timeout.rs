//! Timeout Request Middleware.
//!
//! This middleware applies a timeout to requests processed by the application.
//! The timeout duration is configurable and defined via the [`TimeoutRequestMiddleware`]
//! configuration. The middleware ensures that requests do not run beyond the specified
//! timeout period, improving the overall performance and responsiveness of the application.
//!
//! If a request exceeds the specified timeout duration, the middleware will return
//! a `408 Request Timeout` status code to the client, indicating that the request
//! took too long to process.
//!
use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};
use axum::Router as AXRouter;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

/// Timeout middleware configuration
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct TimeOut {
    pub enable: bool,
    // Timeout request in milliseconds
    pub timeout: u64,
}

impl MiddlewareLayer for TimeOut {
    /// Returns the name of the middleware.
    fn name(&self) -> &'static str {
        "timeout"
    }

    /// Checks if the timeout middleware is enabled.
    fn is_enabled(&self) -> bool {
        self.enable
    }

    /// Applies the timeout middleware to the application router.
    ///
    /// This method wraps the provided [`AXRouter`] in a [`TimeoutLayer`], ensuring
    /// that requests exceeding the specified timeout duration will be interrupted.
    ///
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(TimeoutLayer::new(Duration::from_millis(self.timeout))))
    }
}
