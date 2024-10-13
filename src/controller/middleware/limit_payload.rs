//! Limit Payload Middleware
//!
//! This middleware restricts the maximum allowed size for HTTP request
//! payloads. It is configurable based on the [`LimitPayloadMiddleware`]
//! settings in the application's middleware configuration. The middleware sets
//! a limit on the request body size using Axum's `DefaultBodyLimit` layer.
//!
//! # Note
//!
//! Ensure that the `body: axum::body::Bytes` variable is properly set in the
//! request action to enforce the payload limit correctly. Without this, the
//! middleware will not function as intended.

use axum::Router as AXRouter;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitPayload {
    #[serde(default)]
    pub enable: bool,
    #[serde(deserialize_with = "deserialize_body_limit")]
    #[serde(default = "default_body_limit")]
    pub body_limit: usize,
}

impl Default for LimitPayload {
    fn default() -> Self {
        Self {
            enable: false,
            body_limit: default_body_limit(),
        }
    }
}

fn default_body_limit() -> usize {
    2_000_000
}

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
        self.enable
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
