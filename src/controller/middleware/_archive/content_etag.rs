use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    body::{to_bytes, Body, Bytes},
    extract::Request,
    response::Response,
    BoxError,
};
use futures_util::Future;
use hyper::header::{ETAG, IF_NONE_MATCH};
use sha2::{Digest, Sha256};
use tower::{Layer, Service}; // Corrected import

#[derive(Clone)]
pub struct EtagLayer;

impl EtagLayer {
    pub fn new() -> Self {
        Self {}
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
    S: Service<Request<Body>, Response = Response<Body>>,
    S::Response: 'static,
    S::Error: Into<BoxError> + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let ifnm = request.headers().get(IF_NONE_MATCH).cloned();
        // TODO:
        // handle case where headers already have etag header because some other
        // middleware added it or someone added it manually, and short-circuit
        // the comparison and bail
        // then split this into 2 in config
        //      <etag route - doesnt exist yet>
        //      etag_response: true
        //        regex for which routes to do this on
        //
        //      etag: true
        //       looks for the etag header itself, has to appear in the end
        //
        let future = self.inner.call(request);
        let res_fut = async move {
            let response = future.await.map_err(Into::into)?;
            let (parts, body) = response.into_parts();
            to_bytes(body, 5_000_000)
                .await
                .and_then(|bytes| {
                    let etag = calculate_etag(&bytes);
                    let response = Response::from_parts(parts, Body::from(bytes));

                    if let Some(etag_in_request) = ifnm {
                        if etag_in_request == &etag {
                            return Ok(Response::builder()
                                .status(304)
                                .body(Body::empty())
                                .unwrap());
                        }
                    }

                    let mut response_with_etag = response;
                    response_with_etag
                        .headers_mut()
                        .insert(ETAG, etag.parse().unwrap());
                    Ok(response_with_etag)
                })
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
        };
        Box::pin(res_fut)
    }
}

fn calculate_etag(bytes: &Bytes) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// Usage in Axum application setup remains the same
