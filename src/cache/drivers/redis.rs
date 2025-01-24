//! # Redis Cache Driver
//!
//! This module implements a cache driver using an redis cache.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use bb8::Pool;
use moka::{sync::Cache, Expiry};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sidekiq::RedisConnectionManager;
use super::CacheDriver;
use crate::cache::CacheResult;
use crate::config::RedisCacheConfig;

/// Creates a new instance of the in-memory cache driver, with a default Loco
/// configuration.
///
/// # Returns
///
/// A boxed [`CacheDriver`] instance.
#[must_use]
pub async fn new(config: &RedisCacheConfig) -> Box<dyn CacheDriver> {
    let manager = RedisConnectionManager::new(config.uri.clone())?;
    let redis = Pool::builder().build(manager).await?;

    todo!()
}

/// Represents the in-memory cache driver.
#[derive(Debug)]
pub struct Redis {
    cache: Cache<String, (Expiration, dyn Serialize)>,
}

impl Redis {
    /// Constructs a new [`Redis`] instance from a given cache.
    ///
    /// # Returns
    ///
    /// A boxed [`CacheDriver`] instance.
    #[must_use]
    pub fn from(cache: Cache<String, (Expiration, String)>) -> Box<dyn CacheDriver> {
        Box::new(Self { cache })
    }
}

#[async_trait]
impl CacheDriver for Redis {
    /// Checks if a key exists in the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn contains_key(&self, key: &str) -> CacheResult<bool> {
        Ok(self.cache.contains_key(key))
    }

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn get<T: DeserializeOwned>(&self, key: &str) -> CacheResult<Option<T>> {
        let result = self.cache.get(key);
        match result {
            None => Ok(None),
            Some(v) => Ok(Some(v.1)),
        }
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn insert<T: Serialize>(&self, key: &str, value: &T) -> CacheResult<()> {
        self.cache.insert(
            key.to_string(),
            (Expiration::Never, Arc::new(value).to_string()),
        );
        Ok(())
    }

    /// Inserts a key-value pair into the cache that expires after the specified
    /// number of seconds.
    ///
    /// # Errors
    ///
    /// Returns a [`super::CacheError`] if there is an error during the
    /// operation.
    async fn insert_with_expiry<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        duration: Duration,
    ) -> CacheResult<()> {
        self.cache.insert(
            key.to_string(),
            (
                Expiration::AfterDuration(duration),
                Arc::new(value).to_string(),
            ),
        );
        Ok(())
    }

    /// Removes a key-value pair from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn remove(&self, key: &str) -> CacheResult<()> {
        self.cache.remove(key);
        Ok(())
    }

    /// Clears all key-value pairs from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn clear(&self) -> CacheResult<()> {
        self.cache.invalidate_all();
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Expiration {
    Never,
    AfterDuration(Duration),
}

impl Expiration {
    #[must_use]
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            Self::Never => None,
            Self::AfterDuration(d) => Some(*d),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn is_contains_key() {
        todo!()
    }

    #[tokio::test]
    async fn can_get_key_value() {
        todo!()
    }

    #[tokio::test]
    async fn can_remove_key() {
        todo!()
    }

    #[tokio::test]
    async fn can_clear() {
        todo!()
    }
}
