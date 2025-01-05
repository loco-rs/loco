pub mod driver;
pub mod layer;

use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::Key;
use serde::de::DeserializeOwned;
use tower_sessions::{
    session::{Id, Record},
    SessionStore,
};

use crate::{
    controller::middleware::{self, request_id::LocoRequestId},
    prelude,
    request_context::driver::{Driver, DriverError},
};

/// Enum representing errors that can occur in the `RequestContext` module.
///
/// # Errors
/// - `ConfigurationError`: Indicates a configuration error.
/// - `SignedPrivateCookieJarError`: Represents an error from the signed private
///   cookie jar.
/// - `DriverError`: Indicates an error from the driver module.
#[derive(thiserror::Error, Debug)]
pub enum RequestContextError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    // Convert Signed private cookie jar error
    #[error("Signed private cookie jar error: {0}")]
    SignedPrivateCookieJarError(#[from] driver::cookie::SignedPrivateCookieJarError),
    // Convert Driver Errors
    #[error("Driver error: {0}")]
    DriverError(#[from] DriverError),
}

/// Defines a `RequestContextStore` struct that holds a private key and a
/// configuration for request context sessions.
///
/// # Fields
/// - `private_key`: Key - Private key for the `RequestContextStore`.
/// - `config`: `RequestContextSession` - Configuration for the request context
///   session.
#[derive(Debug, Clone)]
pub struct RequestContextStore {
    private_key: Key,
    session_config: middleware::request_context::RequestContextSession,
    session_cookie_config: middleware::request_context::SessionCookieConfig,
}

impl RequestContextStore {
    /// Create a new instance of the `RequestContextStore`.
    ///
    /// # Arguments
    /// - `private_key`: Key - Private key for the `RequestContextStore`.
    /// - `config::RequestContextSession` - Configuration for the request
    ///   context session.
    ///
    /// # Return
    /// - `Self` - The new instance of the `RequestContextStore`.
    #[must_use]
    pub fn new(
        private_key: Key,
        session_config: middleware::request_context::RequestContextSession,
        session_cookie_config: middleware::request_context::SessionCookieConfig,
    ) -> Self {
        Self {
            private_key,
            session_config,
            session_cookie_config,
        }
    }
}

/// Defines a `CustomSessionStore` struct to hold a `SessionStore`
#[derive(Debug, Clone)]
pub struct TowerSessionStore {
    inner: Arc<dyn SessionStore + Send + Sync>,
}

impl TowerSessionStore {
    #[must_use]
    pub fn new<S>(inner: S) -> Self
    where
        S: SessionStore + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(inner),
        }
    }
}
#[async_trait]
impl SessionStore for TowerSessionStore {
    async fn create(
        &self,
        session_record: &mut Record,
    ) -> tower_sessions::session_store::Result<()> {
        self.inner.create(session_record).await
    }

    async fn save(&self, session_record: &Record) -> tower_sessions::session_store::Result<()> {
        self.inner.save(session_record).await
    }

    async fn load(&self, session_id: &Id) -> tower_sessions::session_store::Result<Option<Record>> {
        self.inner.load(session_id).await
    }

    async fn delete(&self, session_id: &Id) -> tower_sessions::session_store::Result<()> {
        self.inner.delete(session_id).await
    }
}

/// Defines a `RequestContext` struct that holds a [`LocoRequestId`] and a
/// [`Driver`].
///
/// # Fields
/// - [`LocoRequestId`] - The request id for the request context.
/// - [`Driver`] - The driver for the request context.
#[derive(Debug, Clone)]
pub struct RequestContext {
    request_id: LocoRequestId,
    driver: Driver,
}

impl RequestContext {
    /// Create a new instance of the `RequestContext`.
    ///
    /// # Arguments
    /// - [`LocoRequestId`] - The request id for the request context.
    /// - [`Driver`] - The driver for the request context.
    ///
    /// # Return
    /// - [`Self`] - The new instance of the [`RequestContext`].
    #[must_use]
    pub fn new(request_id: LocoRequestId, driver: Driver) -> Self {
        Self { request_id, driver }
    }

    /// Returns the `LocoRequestId` for the current request context.
    ///
    /// # Return
    /// - `&LocoRequestId` - The request id for the request context.
    #[must_use]
    pub fn request_id(&self) -> &LocoRequestId {
        &self.request_id
    }

    /// Inserts a `impl Serialize` value into the session.
    /// # Arguments
    /// * `key` - The key to store the value
    /// * `value` - The value to store
    /// # Errors
    /// * [`CookieMapError`] - When the value is unable to be serialized
    /// * [`TowerSessionError`] - When the value is unable to be serialized or
    ///   if the session has not been hydrated and loading from the store fails,
    ///   we fail with `Error::Store`
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
    /// * [`Option<T>`] - The value if it exists
    /// # Errors
    /// * [`CookieMapError`] - When the value is unable to be deserialized
    /// * [`TowerSessionError`] - When the value is unable to be deserialized or
    ///   if the session has not been hydrated and loading from the store fails,
    ///   we fail with `Error::Store`
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
    /// * [`Option<T>`] - The value if it exists
    ///
    /// # Errors
    /// * [`CookieMapError`] - When the value is unable to be deserialized
    /// * [`TowerSessionError`] - When the value is unable to be deserialized or
    ///   if the session has not been hydrated and loading from the store fails,
    ///   we fail with `Error::Store`
    pub async fn remove<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, RequestContextError> {
        self.driver
            .remove(key)
            .await
            .map_err(RequestContextError::DriverError)
    }

    /// Tower - Clears the session but not the session store.
    /// Cookie - Clear the session map.
    pub async fn clear(&mut self) {
        self.driver.clear().await;
    }

    /// Tower - Flush the session store.
    /// Cookie - Clear the session map.
    ///
    /// # Returns
    /// * `()`
    ///
    /// # Errors
    /// * [`TowerSessionError`] - When the session store fails to flush
    pub async fn flush(&mut self) -> Result<(), RequestContextError> {
        self.driver
            .flush()
            .await
            .map_err(RequestContextError::DriverError)
    }
}

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
