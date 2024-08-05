pub mod request_id;

use crate::config;
use crate::prelude::IntoResponse;
use crate::request_context::driver::cookie::SignedPrivateCookieJar;
use crate::request_context::driver::Driver;
use crate::request_context::layer::request_id::RequestId;
use crate::request_context::{RequestContext, RequestContextError, RequestContextStore};
use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use tower::{Layer, Service};
#[derive(Debug, Clone)]
pub struct RequestContextLayer {
    pub store: Arc<RequestContextStore>,
}

impl RequestContextLayer {
    #[must_use]
    pub fn new(request_context_store: RequestContextStore) -> Self {
        Self {
            store: Arc::new(request_context_store),
        }
    }
}

impl<S> Layer<S> for RequestContextLayer {
    type Service = RequestContextService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            store: self.store.clone(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct RequestContextService<S> {
    inner: S,
    store: Arc<RequestContextStore>,
}

impl<S> Service<Request<Body>> for RequestContextService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
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

    fn call(&mut self, mut request: Request) -> Self::Future {
        let store = self.store.clone();
        // Because the inner service can panic until ready, we need to ensure we only
        // use the ready service.
        //
        // See: https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let Some(request_id) = request.extensions().get::<RequestId>().cloned() else {
                // In practice this should never happen because we wrap `RequestId`
                // directly.
                tracing::error!("missing request_id request extension");
                return Ok(Response::default());
            };
            match store.config {
                config::RequestContextSession::Cookie { .. } => {
                    let jar =
                        SignedPrivateCookieJar::new(request.headers(), store.private_key.clone());
                    let cookie_map = jar.into_cookie_map().map_err(|e| {
                        tracing::error!(error=?e, "Failed to extract data from cookie jar");
                        let err: crate::Error =
                            RequestContextError::SignedPrivateCookieJarError(e).into();
                        err
                    });
                    let cookie_map = match cookie_map {
                        Ok(cookie_map) => cookie_map,
                        Err(e) => {
                            return Ok(e.into_response());
                        }
                    };
                    let cookie_map = Arc::new(Mutex::new(cookie_map));
                    let driver = Driver::CookieMap(cookie_map.clone());
                    let request_context = RequestContext::new(request_id.clone(), driver);
                    request.extensions_mut().insert(request_context);
                    let mut response: Response = inner.call(request).await?;

                    let jar = SignedPrivateCookieJar::from_cookie_map(
                        &store.private_key,
                        cookie_map.lock().await.clone(),
                    )
                    .map_err(|e| {
                        tracing::error!(error=?e, "Failed to extract data from cookie jar");
                        let err: crate::Error =
                            RequestContextError::SignedPrivateCookieJarError(e).into();
                        err
                    })
                    .map_err(axum::response::IntoResponse::into_response);
                    let jar = match jar {
                        Ok(jar) => jar,
                        Err(e) => {
                            return Ok(e.into_response());
                        }
                    };
                    if let Some(jar) = jar {
                        response = (jar, response).into_response();
                    }
                    Ok(response)
                } // config::RequestContext::Tower { .. } => {
                  //     // This is a placeholder for when we implement the tower session driver.
                  // }
            }
        })
    }
}
