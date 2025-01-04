use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{body::Body, extract::Request, response::Response};
use tokio::sync::Mutex;
use tower::{Layer, Service};
use tower_sessions::Session;
use tracing::Instrument;

use crate::{
    controller::middleware::{self, request_id::LocoRequestId},
    prelude::IntoResponse,
    request_context::{
        driver::{cookie::SignedPrivateCookieJar, Driver},
        RequestContext, RequestContextError, RequestContextStore,
    },
};

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
        let span = tracing::debug_span!("RequestContextService::call");
        // Because the inner service can panic until ready, we need to ensure we only
        // use the ready service.
        //
        // See: https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(
            async move {
                let Some(request_id) = request.extensions().get::<LocoRequestId>().cloned() else {
                    // In practice this should never happen because we wrap `RequestId`
                    // directly.
                    tracing::error!("missing request_id request extension");
                    return Ok(Response::default());
                };
                match store.session_config {
                    middleware::request_context::RequestContextSession::Cookie { .. } => {
                        let jar = match SignedPrivateCookieJar::new(
                            request.headers(),
                            store.private_key.clone(),
                        ) {
                            Ok(jar) => jar,
                            Err(e) => {
                                tracing::error!(error=?e, "Failed to create signed private cookie jar");
                                let err: crate::Error =
                                    RequestContextError::SignedPrivateCookieJarError(e).into();
                                return Ok(err.into_response());
                            }
                        };
                        let cookie_map = jar
                            .into_cookie_map(&store.session_cookie_config.clone())
                            .map_err(|e| {
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
                            &store.session_cookie_config.clone(),
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
                                tracing::error!(error=?e, "Failed to extract data from cookie jar");
                                return Ok(e.into_response());
                            }
                        };
                        if let Some(jar) = jar {
                            response = (jar, response).into_response();
                        } else {
                            tracing::error!("Cannot find cookie jar from request context");
                        }
                        Ok(response)
                    }
                    middleware::request_context::RequestContextSession::Tower => {
                        let Some(session) = request.extensions().get::<Session>().cloned() else {
                            // In practice this should never happen because we wrap `Session`
                            // directly.
                            tracing::error!(
                                "cannot get session from request extension in request context \
                                 layer, this happened because there is no tower session layer \
                                 before request context layer in the app"
                            );
                            return Ok(Response::default());
                        };

                        let request_context =
                            RequestContext::new(request_id.clone(), Driver::TowerSession(session));
                        request.extensions_mut().insert(request_context);
                        // This is a placeholder for when we implement the tower session driver.
                        let response: Response = inner.call(request).await?;

                        Ok(response)
                    }
                }
            }
            .instrument(span),
        )
    }
}
