//! Powered-By Middleware
//!
//! This middleware injects an HTTP header `X-Powered-By` into the response headers of
//! every request handled by the application. The header identifies the software or technology
//! stack powering the application. It supports a custom identifier string or defaults to "loco.rs"
//! if no identifier is provided.

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};
use axum::{
    http::header::{HeaderName, HeaderValue},
    Router as AXRouter,
};
use tower_http::set_header::SetResponseHeaderLayer;

lazy_static::lazy_static! {
    static ref DEFAULT_IDENT_HEADER_VALUE: HeaderValue =
        HeaderValue::from_static("loco.rs");
}

/// [`Middleware`] struct responsible for managing the identifier value for the `X-Powered-By` header.
pub struct Middleware {
    ident: Option<HeaderValue>,
}

/// Creates a new instance of [`Middleware`] by cloning the [`Config`] configuration.
#[must_use]
pub fn new(ident: Option<&str>) -> Middleware {
    let ident_value = ident.map_or_else(
        || Some(DEFAULT_IDENT_HEADER_VALUE.clone()),
        |ident| {
            if ident.is_empty() {
                None
            } else {
                match HeaderValue::from_str(ident) {
                    Ok(val) => Some(val),
                    Err(e) => {
                        tracing::info!(
                            error = format!("{}", e),
                            val = ident,
                            "could not set custom ident header"
                        );
                        Some(DEFAULT_IDENT_HEADER_VALUE.clone())
                    }
                }
            }
        },
    );

    Middleware { ident: ident_value }
}

impl MiddlewareLayer for Middleware {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "powered_by"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.ident.is_some()
    }

    /// Applies the middleware to the application by adding the `X-Powered-By` header to
    /// each response.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-powered-by"),
            self.ident
                .clone()
                .unwrap_or_else(|| DEFAULT_IDENT_HEADER_VALUE.clone()),
        )))
    }
}
