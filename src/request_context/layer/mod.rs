use crate::request_context::RequestContextStore;
use axum::http::{Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

pub struct RequestContextLayer {
    pub request_context_store: RequestContextStore,
}
impl<S> Layer<S> for RequestContextLayer {
    type Service = RequestContextService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            store: self.request_context_store.clone(),
        }
    }
}

pub struct RequestContextService<S> {
    inner: S,
    store: RequestContextStore,
}

impl<ReqBody, ResBody, S> Service<Request<ReqBody>> for RequestContextService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<ReqBody>) -> Self::Future {
        // Because the inner service can panic until ready, we need to ensure we only
        // use the ready service.
        //
        // See: https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let response = inner.call(request).await?;
            Ok(response)
        })
    }
}
