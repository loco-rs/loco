//! # Null Cache Driver
//!
//! The Null Cache Driver is the default cache driver implemented when the Loco
//! framework is initialized. The primary purpose of this driver is to simplify
//! the user workflow by avoiding the need for feature flags or optional cache
//! driver configurations.
use std::time::Duration;

use async_trait::async_trait;

use super::CacheDriver;
use crate::cache::{CacheError, CacheResult};

/// Represents the in-memory cache driver.
#[derive(Debug)]
pub struct Null {}

/// Creates a new null cache instance
///
/// # Returns
///
/// A boxed [`CacheDriver`] instance.
#[must_use]
pub fn new() -> Box<dyn CacheDriver> {
    Box::new(Null {})
}

#[async_trait]
impl CacheDriver for Null {
    /// Pings the cache to check if it is reachable.
    ///
    /// # Errors
    ///
    /// Returns always error
    async fn ping(&self) -> CacheResult<()> {
        Err(CacheError::Any(
            "Operation not supported by null cache".into(),
        ))
    }

    /// Checks if a key exists in the cache.
    ///
    /// # Errors
    ///
    /// Returns always error
    async fn contains_key(&self, _key: &str) -> CacheResult<bool> {
        Err(CacheError::Any(
            "Operation not supported by null cache".into(),
        ))
    }

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Errors
    ///
    /// Returns always error
    async fn get(&self, _key: &str) -> CacheResult<Option<String>> {
        Ok(None)
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// # Errors
    ///
    /// Returns always error
    async fn insert(&self, _key: &str, _value: &str) -> CacheResult<()> {
        Err(CacheError::Any(
            "Operation not supported by null cache".into(),
        ))
    }

    /// Inserts a key-value pair into the cache that expires after the
    /// provided duration.
    ///
    /// # Errors
    ///
    /// Returns always error
    async fn insert_with_expiry(
        &self,
        _key: &str,
        _value: &str,
        _duration: Duration,
    ) -> CacheResult<()> {
        Err(CacheError::Any(
            "Operation not supported by null cache".into(),
        ))
    }

    /// Removes a key-value pair from the cache.
    ///
    /// # Errors
    ///
    /// Returns always error
    async fn remove(&self, _key: &str) -> CacheResult<()> {
        Err(CacheError::Any(
            "Operation not supported by null cache".into(),
        ))
    }

    /// Clears all key-value pairs from the cache.
    ///
    /// # Errors
    ///
    /// Returns always error
    async fn clear(&self) -> CacheResult<()> {
        Err(CacheError::Any(
            "Operation not supported by null cache".into(),
        ))
    }
}
