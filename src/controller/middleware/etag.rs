//! etag middleware for caching requests. See [ETag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)
use std::task::{Context, Poll};

use axum::{body::Body, extract::Request, response::Response};
use futures_util::future::BoxFuture;
use hyper::header::{ETAG, IF_NONE_MATCH};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct EtagLayer;

impl EtagLayer {
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for EtagLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for EtagLayer {
    type Service = EtagMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        EtagMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct EtagMiddleware<S> {
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
            let response = future.await.map_err(Into::into)?;
            let etag_from_response = response.headers().get(ETAG).cloned();
            if let Some(etag_in_request) = ifnm {
                if let Some(etag_from_response) = etag_from_response {
                    if etag_in_request == etag_from_response {
                        return Ok(Response::builder().status(304).body(Body::empty()).unwrap());
                    }
                }
            }
            Ok(response)
        };
        Box::pin(res_fut)
    }
}
