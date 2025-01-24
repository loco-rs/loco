//! # Redis Cache Driver
//!
//! This module implements a cache driver using an redis cache.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::Serialize;
use super::CacheDriver;
use crate::cache::{CacheError, CacheResult};
use crate::config::RedisCacheConfig;

/// Creates a new instance of the in-memory cache driver, with a default Loco
/// configuration.
///
/// # Returns
///
/// A boxed [`CacheDriver`] instance.
#[must_use]
pub async fn new(config: &RedisCacheConfig) -> CacheResult<Box<dyn CacheDriver>> {
    let manager = RedisConnectionManager::new(config.uri.clone())
        .map_err(|e| CacheError::Any(Box::new(e)))?;
    let redis = Pool::builder().build(manager).await?;

    Ok(Box::new(Redis { redis }))
}

/// Represents the in-memory cache driver.
pub struct Redis {
    redis: Pool<RedisConnectionManager>,
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
        let mut connection = self.redis.get().await?;
        Ok(connection.exists(key).await?)

    }

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn get<T: DeserializeOwned>(&self, key: &str) -> CacheResult<Option<T>> {
        let mut connection = self.redis.get().await?;
        let data: Option<Vec<u8>> = connection.get(key).await?;

        match data {
            Some(bytes) => {
                let value = rmp_serde::from_slice(&bytes)
                    .map_err(|e| CacheError::Any(Box::new(e)))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn insert<T: Serialize>(&self, key: &str, value: &T) -> CacheResult<()> {
        let mut connection = self.redis.get().await?;
        let encoded = rmp_serde::to_vec(value)
            .map_err(|e| CacheError::Any(Box::new(e)))?;
        connection.set(key, encoded).await?;
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
        let mut connection = self.redis.get().await?;
        let encoded = rmp_serde::to_vec(value)
            .map_err(|e| CacheError::Any(Box::new(e)))?;
        connection.set_ex(key, encoded, duration.as_secs() as usize).await?;
        Ok(())
    }

    /// Removes a key-value pair from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn remove(&self, key: &str) -> CacheResult<()> {
        let mut connection = self.redis.get().await?;
        connection.del(key);
        Ok(())
    }

    /// Clears all key-value pairs from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn clear(&self) -> CacheResult<()> {
        let mut connection = self.redis.get().await?;
        connection.flushdb().await?;

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
