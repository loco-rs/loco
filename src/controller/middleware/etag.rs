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
        header::{ETAG, IF_NONE_MATCH},
        StatusCode,
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
        let ifnm = request.headers().get(IF_NONE_MATCH).cloned();

        let future = self.inner.call(request);

        let res_fut = async move {
            let response = future.await?;
            let etag_from_response = response.headers().get(ETAG).cloned();
            if let Some(etag_in_request) = ifnm {
                if let Some(etag_from_response) = etag_from_response {
                    if etag_in_request == etag_from_response {
                        return Ok(Response::builder()
                            .status(StatusCode::NOT_MODIFIED)
                            .body(Body::empty())
                            .unwrap());
                    }
                }
            }
            Ok(response)
        };
        Box::pin(res_fut)
    }
}
