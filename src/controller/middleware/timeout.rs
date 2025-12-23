//! Timeout Request Middleware.
//!
//! This middleware applies a timeout to requests processed by the application.
//! The timeout duration is configurable and defined via the
//! [`TimeoutRequestMiddleware`] configuration. The middleware ensures that
//! requests do not run beyond the specified timeout period, improving the
//! overall performance and responsiveness of the application.
//!
//! If a request exceeds the specified timeout duration, the middleware will
//! return a `408 Request Timeout` status code to the client, indicating that
//! the request took too long to process.
use std::time::Duration;

use axum::{http::StatusCode, Router as AXRouter};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::timeout::TimeoutLayer;

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

/// Timeout middleware configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeOut {
    #[serde(default)]
    pub enable: bool,
    // Timeout request in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl Default for TimeOut {
    fn default() -> Self {
        serde_json::from_value(json!({})).unwrap()
    }
}

fn default_timeout() -> u64 {
    5_000
}

impl MiddlewareLayer for TimeOut {
    /// Returns the name of the middleware.
    fn name(&self) -> &'static str {
        "timeout_request"
    }

    /// Checks if the timeout middleware is enabled.
    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the timeout middleware to the application router.
    ///
    /// This method wraps the provided [`AXRouter`] in a [`TimeoutLayer`],
    /// ensuring that requests exceeding the specified timeout duration will
    /// be interrupted.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_millis(self.timeout),
        )))
    }
}
