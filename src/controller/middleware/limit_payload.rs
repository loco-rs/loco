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

use aide::axum::ApiRouter;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum DefaultBodyLimitKind {
    Disable,
    Limit(usize),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitPayload {
    #[serde(
        default = "default_body_limit",
        deserialize_with = "deserialize_body_limit"
    )]
    pub body_limit: DefaultBodyLimitKind,
}

impl Default for LimitPayload {
    fn default() -> Self {
        Self {
            body_limit: default_body_limit(),
        }
    }
}

/// Returns the default body limit in bytes (2MB).
fn default_body_limit() -> DefaultBodyLimitKind {
    DefaultBodyLimitKind::Limit(2_000_000)
}

fn deserialize_body_limit<'de, D>(deserializer: D) -> Result<DefaultBodyLimitKind, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;

    match s.as_str() {
        "disable" => Ok(DefaultBodyLimitKind::Disable),
        limit => {
            let bytes = byte_unit::Byte::from_str(limit)
                .map_err(|err| serde::de::Error::custom(err.to_string()))?
                .get_bytes();
            Ok(DefaultBodyLimitKind::Limit(bytes as usize))
        }
    }
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
    fn apply(&self, app: ApiRouter<AppContext>) -> Result<ApiRouter<AppContext>> {
        let body_limit_layer = match self.body_limit {
            DefaultBodyLimitKind::Disable => axum::extract::DefaultBodyLimit::disable(),
            DefaultBodyLimitKind::Limit(limit) => axum::extract::DefaultBodyLimit::max(limit),
        };

        Ok(app.layer(body_limit_layer))
    }
}
