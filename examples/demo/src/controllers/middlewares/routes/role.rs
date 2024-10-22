use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use futures_util::future::BoxFuture;
use loco_rs::{
    axum::{
        body::Body,
        extract::{FromRequestParts, Request},
        response::Response,
    },
    prelude::{auth::JWTWithUser, *},
};
use tower::{Layer, Service};

use crate::models::{roles, users};

#[derive(Clone)]
pub struct RoleRouteLayer {
    state: AppContext,
}

impl RoleRouteLayer {
    pub fn new(state: AppContext) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for RoleRouteLayer {
    type Service = RoleRouteService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            state: self.state.clone(),
        }
    }
}
#[derive(Clone)]
pub struct RoleRouteService<S> {
    inner: S,
    state: AppContext,
}

impl<S, B> Service<Request<B>> for RoleRouteService<S>
where
    S: Service<Request<B>, Response = Response<Body>, Error = Infallible> + Clone + Send + 'static, /* Inner Service must return Response<Body> and never error */
    S::Future: Send + 'static,
    B: Send + 'static,
{
    // Response type is the same as the inner service / handler
    type Response = S::Response;
    // Error type is the same as the inner service / handler
    type Error = S::Error;
    // Future type is the same as the inner service / handler
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let state = self.state.clone();
        let clone = self.inner.clone();
        // take the service that was ready
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            // Example of extracting JWT and checking roles
            let (mut parts, body) = req.into_parts();
            let auth = JWTWithUser::<users::Model>::from_request_parts(&mut parts, &state).await;

            match auth {
                Ok(auth) => {
                    // Check user roles here
                    // If the user has the required role, proceed with the inner service
                    let _role = match roles::Model::find_by_user(&state.db, &auth.user).await {
                        Ok(role) => role,
                        Err(_) => {
                            return Ok(Response::builder()
                                .status(401)
                                .body(Body::empty())
                                .unwrap()
                                .into_response())
                        }
                    };
                    let req = Request::from_parts(parts, body);
                    inner.call(req).await
                }
                Err(_) => {
                    // Handle error, e.g., return an unauthorized response
                    Ok(Response::builder()
                        .status(401)
                        .body(Body::empty())
                        .unwrap()
                        .into_response())
                }
            }
        })
    }
}
