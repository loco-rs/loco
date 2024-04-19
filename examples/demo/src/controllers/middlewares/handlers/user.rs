use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::{FromRequestParts, Request},
    response::Response,
};
use futures_util::future::BoxFuture;
use loco_rs::prelude::{auth::JWTWithUser, *};
use tower::{Layer, Service};

use crate::models::{roles, sea_orm_active_enums::RolesName, users};

#[derive(Clone)]
pub struct UserHandlerLayer {
    state: AppContext,
}

impl UserHandlerLayer {
    pub fn new(state: AppContext) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for UserHandlerLayer {
    type Service = UserHandlerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct UserHandlerService<S> {
    inner: S,
    state: AppContext,
}

/// Service that checks if the user has the required user role before calling
/// the inner service If the user has the required role, the inner service is
/// called Otherwise, an unauthorized response is returned
impl<S, B> Service<Request<B>> for UserHandlerService<S>
where
    S: Service<Request<B>, Response = Response<Body>, Error = Infallible> + Clone + Send + 'static,
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
                    let _user = match roles::Model::find_by_user(&state.db, &auth.user).await {
                        Ok(role) => match role.name {
                            RolesName::Admin => {
                                return Ok(Response::builder()
                                    .status(401)
                                    .body(Body::empty())
                                    .unwrap()
                                    .into_response())
                            }
                            RolesName::User => role,
                        },
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
                    // Return an unauthorized response
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
