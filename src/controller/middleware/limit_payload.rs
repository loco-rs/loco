//! # Limit Payload Middleware
//!
//! This middleware restricts the maximum allowed size for HTTP request payloads.
//! It ensures that incoming HTTP requests do not exceed a specified payload size,
//! which helps protect the application from overly large requests that could affect performance.
//!
//! # Note
//!
//! Ensure that the `body: axum::body::Bytes` variable is properly set in the
//! request action to enforce the payload limit correctly. Without this, the
//! middleware will not function as intended.

use axum::Router as AXRouter;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

/// Middleware configuration for limiting payload size.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitPayload {
    #[serde(
        default = "default_body_limit",
        deserialize_with = "deserialize_body_limit"
    )]
    pub body_limit: usize,
}

impl Default for LimitPayload {
    /// Provides the default configuration for the middleware.
    fn default() -> Self {
        Self {
            body_limit: default_body_limit(),
        }
    }
}

/// Returns the default body limit in bytes (50MB).
fn default_body_limit() -> usize {
    50_000_000
}

/// Custom deserialization for `body_limit`, allowing human-readable formats.
fn deserialize_body_limit<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        byte_unit::Byte::from_str(String::deserialize(deserializer)?)
            .map_err(|err| serde::de::Error::custom(err.to_string()))?
            .get_bytes() as usize,
    )
}

impl MiddlewareLayer for LimitPayload {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "limit_payload"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the payload limit middleware to the application router by adding
    /// a `DefaultBodyLimit` layer.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(axum::extract::DefaultBodyLimit::max(self.body_limit)))
    }
}
