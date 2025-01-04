use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{body::Body, extract::Request, http::header, response::Response, Router as AXRouter};
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};
use tracing::Instrument;

use crate::{
    app::AppContext, controller::middleware::MiddlewareLayer, prelude::IntoResponse, Error, Result,
};

const DEFAULT_MAX_SIZE: usize = 4096; // 4KB default max size

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CookieSize {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_max_size")]
    max_size: usize,
}

fn default_max_size() -> usize {
    DEFAULT_MAX_SIZE
}

impl Default for CookieSize {
    fn default() -> Self {
        Self {
            enable: false,
            max_size: DEFAULT_MAX_SIZE,
        }
    }
}

#[derive(Clone)]
struct CookieSizeLayer {
    max_size: usize,
}

impl CookieSizeLayer {
    fn new(max_size: usize) -> Self {
        Self { max_size }
    }
}

impl<S> Layer<S> for CookieSizeLayer {
    type Service = CookieSizeMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CookieSizeMiddleware {
            inner,
            max_size: self.max_size,
        }
    }
}

#[derive(Clone)]
struct CookieSizeMiddleware<S> {
    inner: S,
    max_size: usize,
}

impl<S> Service<Request<Body>> for CookieSizeMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Response: 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let cookies = request.headers().get_all(header::COOKIE);
        let total_size: usize = cookies.iter().map(axum::http::HeaderValue::len).sum();
        let span = tracing::info_span!("CookieSizeMiddleware::call");
        let max_size = self.max_size;

        // Because the inner service can panic until ready, we need to ensure we only
        // use the ready service.
        //
        // See: https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(
            async move {
                if total_size > max_size {
                    let error_msg = format!(
                        "Cookie size {total_size} exceeds maximum allowed size of {max_size} bytes",
                    );
                    return Ok(Error::BadRequest(error_msg).into_response());
                }
                let response = inner.call(request).await?;
                Ok(response)
            }
            .instrument(span),
        )
    }
}

impl MiddlewareLayer for CookieSize {
    fn name(&self) -> &'static str {
        "cookie_size"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(CookieSizeLayer::new(self.max_size)))
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    use super::*;
    use crate::tests_cfg;

    #[tokio::test]
    async fn test_cookie_size_validation() {
        let middleware = CookieSize {
            enable: true,
            max_size: 10,
        };

        let app = Router::new().route("/", get(|| async { "OK" }));

        let app = middleware
            .apply(app)
            .expect("apply middleware")
            .with_state(tests_cfg::app::get_app_context().await);

        // Test with cookie exceeding size limit
        let req = Request::builder()
            .uri("/")
            .method(Method::GET)
            .header(
                header::COOKIE,
                "session=very_long_cookie_value_exceeding_limit",
            )
            .body(Body::empty())
            .expect("request");

        let response = app.clone().oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test with acceptable cookie size
        let req = Request::builder()
            .uri("/")
            .method(Method::GET)
            .header(header::COOKIE, "session=ok")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
