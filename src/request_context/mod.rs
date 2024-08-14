pub mod driver;
pub mod layer;

use crate::controller::middleware::request_id::LocoRequestId;
use crate::request_context::driver::{Driver, DriverError};
use crate::{config, prelude};
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::Key;
use serde::de::DeserializeOwned;

#[derive(thiserror::Error, Debug)]
pub enum RequestContextError {
    /// Configuration error
    #[error("Configuration error")]
    ConfigurationError,
    // Convert Signed private cookie jar error
    #[error("Signed private cookie jar error: {0}")]
    SignedPrivateCookieJarError(#[from] driver::cookie::SignedPrivateCookieJarError),
    // Convert Driver Errors
    #[error("Driver error: {0}")]
    DriverError(#[from] DriverError),
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
    pub fn request_id(&self) -> &LocoRequestId {
        &self.request_id
    }

    /// Inserts a `impl Serialize` value into the session.
    /// # Arguments
    /// * `key` - The key to store the value
    /// * `value` - The value to store
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be serialized
    /// * `TowerSessionError` - When the value is unable to be serialized or if the session has not been hydrated and loading from the store fails, we fail with `Error::Store`
    pub async fn insert<T>(&mut self, key: &str, value: T) -> Result<(), RequestContextError>
    where
        T: serde::Serialize + Send + Sync,
    {
        self.driver
            .insert(key, value)
            .await
            .map_err(RequestContextError::DriverError)
    }

    /// Gets a `impl DeserializeOwned` value from the session.
    /// # Arguments
    /// * `key` - The key to get the value from
    /// # Returns
    /// * `Option<T>` - The value if it exists
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be deserialized
    /// * `TowerSessionError` - When the value is unable to be deserialized or if the session has not been hydrated and loading from the store fails, we fail with `Error::Store`
    pub async fn get<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, RequestContextError> {
        self.driver
            .get(key)
            .await
            .map_err(RequestContextError::DriverError)
    }

    /// Removes a `serde_json::Value` from the session.
    ///
    /// # Arguments
    /// * `key` - The key to remove from the session
    ///
    /// # Return
    /// * `Option<T>` - The value if it exists
    ///
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be deserialized
    /// * `TowerSessionError` - When the value is unable to be deserialized or if the session has not been hydrated and loading from the store fails, we fail with `Error::Store`
    pub async fn remove<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, RequestContextError> {
        self.driver
            .remove(key)
            .await
            .map_err(RequestContextError::DriverError)
    }

    /// Clears the session.
    pub async fn clear(&mut self) {
        self.driver.clear().await;
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
