//! Compression Middleware for Axum
//!
//! This middleware applies compression to HTTP responses to reduce the size of
//! the data being transmitted. This can improve performance by decreasing load
//! times and reducing bandwidth usage. The middleware configuration allows for
//! enabling or disabling compression based on the application settings.
//!
//! The default `compression` feature builds gzip. Enable `compression-full`
//! for brotli, deflate and zstd; the client's `Accept-Encoding` picks among
//! what is built.

use axum::Router as AXRouter;
use serde::{Deserialize, Serialize};
#[cfg(feature = "compression")]
use tower_http::compression::CompressionLayer;

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Compression {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for Compression {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "compression"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the Compression middleware layer to the Axum router.
    #[cfg(feature = "compression")]
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(CompressionLayer::new()))
    }

    /// Without the `compression` feature an enabled `compression` middleware
    /// is a boot error rather than a silent no-op.
    #[cfg(not(feature = "compression"))]
    fn apply(&self, _app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Err(crate::Error::string(
            "server.middlewares.compression is enabled but loco-rs was built without the \
             `compression` feature",
        ))
    }
}
