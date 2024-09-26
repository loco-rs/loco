//! Middleware to generate or ensure a unique request ID for every request.
//! The request ID is stored in the `x-request-id` header, and it is either
//! generated or sanitized if already present in the request.
//!
//! This can be useful for tracking requests across services, logging, and
//! debugging.

use axum::{
    extract::Request, http::HeaderValue, middleware::Next, response::Response, Router as AXRouter,
};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

const X_REQUEST_ID: &str = "x-request-id";
const MAX_LEN: usize = 255;

lazy_static! {
    static ref ID_CLEANUP: Regex = Regex::new(r"[^\w\-@]").unwrap();
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestId {
    enable: bool,
}

impl Default for RequestId {
    fn default() -> Self {
        Self { enable: true }
    }
}

impl MiddlewareLayer for RequestId {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "request_id"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.enable
    }

    /// Applies the request ID middleware to the Axum router.
    ///
    /// This function sets up the middleware in the router and ensures that
    /// every request passing through it will have a unique or sanitized
    /// request ID.
    ///
    /// # Errors
    /// This function returns an error if the middleware cannot be applied.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(axum::middleware::from_fn(request_id_middleware)))
    }
}

/// Wrapper struct for storing the request ID in the request's extensions.
#[derive(Debug, Clone)]
pub struct LocoRequestId(String);

impl LocoRequestId {
    /// Retrieves the request ID as a string slice.
    #[must_use]
    pub fn get(&self) -> &str {
        self.0.as_str()
    }
}

/// Middleware function to ensure or generate a unique request ID.
///
/// This function intercepts requests, checks for the presence of the
/// `x-request-id` header, and either sanitizes its value or generates a new
/// UUID if absent. The resulting request ID is added to both the request
/// extensions and the response headers.
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let header_request_id = request.headers().get(X_REQUEST_ID).cloned();
    let request_id = make_request_id(header_request_id);
    request
        .extensions_mut()
        .insert(LocoRequestId(request_id.clone()));
    let mut res = next.run(request).await;

    if let Ok(v) = HeaderValue::from_str(request_id.as_str()) {
        res.headers_mut().insert(X_REQUEST_ID, v);
    } else {
        tracing::warn!("could not set request ID into response headers: `{request_id}`",);
    }
    res
}

/// Generates or sanitizes a request ID.
fn make_request_id(maybe_request_id: Option<HeaderValue>) -> String {
    maybe_request_id
        .and_then(|hdr| {
            // see: https://github.com/rails/rails/blob/main/actionpack/lib/action_dispatch/middleware/request_id.rb#L39
            let id: Option<String> = hdr.to_str().ok().map(|s| {
                ID_CLEANUP
                    .replace_all(s, "")
                    .chars()
                    .take(MAX_LEN)
                    .collect()
            });
            id.filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;
    use insta::assert_debug_snapshot;

    use super::make_request_id;

    #[test]
    fn create_or_fetch_request_id() {
        let id = make_request_id(Some(HeaderValue::from_static("foo-bar=baz")));
        assert_debug_snapshot!(id);
        let id = make_request_id(Some(HeaderValue::from_static("")));
        assert_debug_snapshot!(id.len());
        let id = make_request_id(Some(HeaderValue::from_static("==========")));
        assert_debug_snapshot!(id.len());
        let long_id = "x".repeat(1000);
        let id = make_request_id(Some(HeaderValue::from_str(&long_id).unwrap()));
        assert_debug_snapshot!(id.len());
        let id = make_request_id(None);
        assert_debug_snapshot!(id.len());
    }
}
