pub mod driver;
pub mod layer;

use crate::controller::middleware::request_id::LocoRequestId;
use crate::request_context::driver::Driver;
use crate::{config, prelude};
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::Key;

#[derive(thiserror::Error, Debug)]
pub enum RequestContextError {
    /// Configuration error
    #[error("Configuration error")]
    ConfigurationError,
    // Convert Signed private cookie jar error
    #[error("Signed private cookie jar error: {0}")]
    SignedPrivateCookieJarError(#[from] driver::cookie::SignedPrivateCookieJarError),
}

#[derive(Debug, Clone)]
pub struct RequestContextStore {
    private_key: Key,
    config: config::RequestContextSession,
}

impl RequestContextStore {
    #[must_use]
    pub fn new(private_key: Key, config: config::RequestContextSession) -> Self {
        Self {
            private_key,
            config,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    request_id: LocoRequestId,
    driver: Driver,
}

impl RequestContext {
    #[must_use]
    pub fn new(request_id: LocoRequestId, driver: Driver) -> Self {
        Self { request_id, driver }
    }

    #[must_use]
    pub fn driver(&mut self) -> &mut Driver {
        &mut self.driver
    }

    #[must_use]
    pub fn request_id(&self) -> &LocoRequestId {
        &self.request_id
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = prelude::Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
            tracing::error!("Failed to extract data from cookie jar");
            prelude::Error::InternalServerError
        })
    }
}
