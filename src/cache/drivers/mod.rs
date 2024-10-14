//! # Cache Drivers Module
//!
//! This module defines traits and implementations for cache drivers.
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::CacheResult;

#[cfg(feature = "cache_inmem")]
pub mod inmem;
pub mod null;

/// Trait representing a cache driver.
#[async_trait]
pub trait CacheDriver: Sync + Send {
    /// The type used for cache keys. Must be serializable and deserializable.
    type Key: Serialize + for<'de> Deserialize<'de> + Send + Sync;

    /// The type used for cache values. Must be serializable and deserializable.
    type Value: Serialize + for<'de> Deserialize<'de> + Send + Sync;
    /// Checks if a key exists in the cache.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn contains_key(&self, key: &Self::Key) -> CacheResult<bool>;

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn get(&self, key: &Self::Key) -> CacheResult<Option<Self::Value>>;

    /// Inserts a key-value pair into the cache.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn insert(&self, key: &Self::Key, value: &Self::Value) -> CacheResult<()>;

    /// Removes a key-value pair from the cache.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn remove(&self, key: &Self::Key) -> CacheResult<()>;

    /// Clears all key-value pairs from the cache.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn clear(&self) -> CacheResult<()>;
}
