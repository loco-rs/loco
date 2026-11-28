//! `ETag` Middleware for Caching Requests
//!
//! This middleware implements the [ETag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)
//! HTTP header for caching responses in Axum. `ETags` are used to validate
//! cache entries by comparing a client's stored `ETag` with the one generated
//! by the server. If the `ETags` match, a `304 Not Modified` response is sent,
//! avoiding the need to resend the full content.

use std::task::{Context, Poll};

use axum::{
    body::Body,
    extract::Request,
    http::{
        header::{
            CACHE_CONTROL, CONTENT_LOCATION, DATE, ETAG, EXPIRES, IF_NONE_MATCH, SET_COOKIE, VARY,
        },
        HeaderValue, Method, StatusCode,
    },
    response::Response,
    Router as AXRouter,
};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Etag {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for Etag {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "etag"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the `ETag` middleware to the application router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(EtagLayer))
    }
}

/// Whether `If-None-Match` names the response's `etag` (RFC 9110 §13.1.2): `*`,
/// or a list of entity tags compared weakly (`W/"x"` matches `"x"`). A header
/// equal to the tag as a whole still matches, so unquoted tags keep working.
fn none_match(if_none_match: &HeaderValue, etag: &HeaderValue) -> bool {
    if if_none_match == etag {
        return true;
    }
    let (Ok(list), Ok(etag)) = (if_none_match.to_str(), etag.to_str()) else {
        return false;
    };
    let opaque = |tag: &str| {
        let tag = tag.trim();
        tag.strip_prefix("W/").unwrap_or(tag).to_string()
    };
    let etag = opaque(etag);
    list.trim() == "*" || list.split(',').any(|tag| opaque(tag) == etag)
}

/// [`EtagLayer`] struct for adding `ETag` functionality as a Tower service
/// layer.
#[derive(Default, Clone)]
struct EtagLayer;

impl<S> Layer<S> for EtagLayer {
    type Service = EtagMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        EtagMiddleware { inner }
    }
}

#[derive(Clone)]
struct EtagMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for EtagMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // RFC 9110 §13.1.2: a matching `If-None-Match` answers 304 only for GET and
        // HEAD. For any other method the handler has already run, so its response
        // must not be replaced (412 would claim the request was not performed).
        let ifnm = matches!(*request.method(), Method::GET | Method::HEAD)
            .then(|| request.headers().get(IF_NONE_MATCH).cloned())
            .flatten();

        let future = self.inner.call(request);

        let res_fut = async move {
            let response = future.await?;
            let etag_from_response = response.headers().get(ETAG).cloned();
            if let Some(etag_in_request) = ifnm
                && let Some(etag_from_response) = etag_from_response
                && response.status().is_success()
                && none_match(&etag_in_request, &etag_from_response)
            {
                // RFC 7232 §4.1: a 304 response must carry the header fields that a
                // 200 would have sent for cache validation. Re-emit the validator and
                // freshness headers from the inner response rather than dropping them.
                let mut builder = Response::builder().status(StatusCode::NOT_MODIFIED);
                if let Some(headers) = builder.headers_mut() {
                    for name in [ETAG, CACHE_CONTROL, VARY, EXPIRES, DATE, CONTENT_LOCATION] {
                        if let Some(value) = response.headers().get(&name) {
                            headers.insert(name, value.clone());
                        }
                    }
                    // Cookies are not part of the representation: a session or
                    // remember-me rotation rides on this response too, and losing
                    // it would leave the browser holding a stale token.
                    for value in response.headers().get_all(SET_COOKIE) {
                        headers.append(SET_COOKIE, value.clone());
                    }
                    // A 304 that sets a cookie must not sit in a shared cache.
                    if headers.contains_key(SET_COOKIE) {
                        let kept = headers
                            .get(CACHE_CONTROL)
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_ascii_lowercase);
                        let private = kept
                            .as_deref()
                            .is_some_and(|v| v.contains("private") || v.contains("no-store"));
                        if !private {
                            let value = kept
                                .map_or_else(|| "private".to_string(), |v| format!("{v}, private"));
                            if let Ok(value) = value.parse() {
                                headers.insert(CACHE_CONTROL, value);
                            }
                        }
                    }
                }
                return Ok(builder.body(Body::empty()).unwrap());
            }
            Ok(response)
        };
        Box::pin(res_fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(if_none_match: &'static str, etag: &'static str) -> bool {
        none_match(
            &HeaderValue::from_static(if_none_match),
            &HeaderValue::from_static(etag),
        )
    }

    #[test]
    fn if_none_match_follows_rfc_9110() {
        assert!(matches("\"a\"", "\"a\""));
        assert!(matches("loco-etag", "loco-etag"));
        assert!(matches("\"x\", \"a\"", "\"a\""));
        assert!(matches("*", "\"a\""));
        assert!(matches("W/\"a\"", "\"a\""));
        assert!(matches("\"a\"", "W/\"a\""));
        assert!(!matches("\"b\"", "\"a\""));
        assert!(!matches("\"x\", \"y\"", "\"a\""));
    }
}
