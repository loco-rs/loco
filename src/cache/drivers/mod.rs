//! # Cache Drivers Module
//!
//! This module defines traits and implementations for cache drivers.
use std::time::Duration;

use async_trait::async_trait;

use super::CacheResult;

#[cfg(feature = "cache_inmem")]
pub mod inmem;
pub mod null;
#[cfg(feature = "cache_redis")]
pub mod redis;

/// Trait representing a cache driver.
#[async_trait]
pub trait CacheDriver: Sync + Send {
    /// Pings the cache to check if it is reachable.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn ping(&self) -> CacheResult<()>;

    /// Checks if a key exists in the cache.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn contains_key(&self, key: &str) -> CacheResult<bool>;

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn get(&self, key: &str) -> CacheResult<Option<String>>;

    /// Inserts a key-value pair into the cache.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn insert(&self, key: &str, value: &str) -> CacheResult<()>;

    /// Inserts a key-value pair into the cache that expires after the
    /// specified duration.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn insert_with_expiry(
        &self,
        key: &str,
        value: &str,
        duration: Duration,
    ) -> CacheResult<()>;

    /// Removes a key-value pair from the cache.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn remove(&self, key: &str) -> CacheResult<()>;

    /// Clears all key-value pairs from the cache.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn clear(&self) -> CacheResult<()>;
}
