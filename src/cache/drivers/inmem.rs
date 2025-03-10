//! # In-Memory Cache Driver
//!
//! This module implements a cache driver using an in-memory cache.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use super::CacheDriver;
use crate::cache::CacheResult;
use crate::config::InMemCacheConfig;
use async_trait::async_trait;
use moka::{sync::Cache, Expiry};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Creates a new instance of the in-memory cache driver, with a default Loco
/// configuration.
///
/// # Returns
///
/// A boxed [`CacheDriver`] instance.
#[must_use]
pub async fn new(config: &InMemCacheConfig) -> Box<dyn CacheDriver> {
    let cache: Cache<String, (Expiration, String)> = Cache::builder()
        .max_capacity(config.max_capacity)
        .expire_after(InMemExpiry)
        .build();
    Inmem::from(cache)
}

/// Represents the in-memory cache driver.
#[derive(Debug)]
pub struct Inmem {
    cache: Cache<String, (Expiration, dyn Serialize)>,
}

impl Inmem {
    /// Constructs a new [`Inmem`] instance from a given cache.
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
impl CacheDriver for Inmem {
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

pub struct InMemExpiry;

impl Expiry<String, (Expiration, String)> for InMemExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &(Expiration, String),
        _current_time: Instant,
    ) -> Option<Duration> {
        value.0.as_duration()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn is_contains_key() {
        let mem = new();
        assert!(!mem.contains_key("key").await.unwrap());
        assert!(mem.insert("key", "loco").await.is_ok());
        assert!(mem.contains_key("key").await.unwrap());
    }

    #[tokio::test]
    async fn can_get_key_value() {
        let mem = new();
        assert!(mem.insert("key", "loco").await.is_ok());
        assert_eq!(mem.get("key").await.unwrap(), Some("loco".to_string()));

        //try getting key that not exists
        assert_eq!(mem.get("not-found").await.unwrap(), None);
    }

    #[tokio::test]
    async fn can_remove_key() {
        let mem = new();
        assert!(mem.insert("key", "loco").await.is_ok());
        assert!(mem.contains_key("key").await.unwrap());
        mem.remove("key").await.unwrap();
        assert!(!mem.contains_key("key").await.unwrap());
    }

    #[tokio::test]
    async fn can_clear() {
        let mem = new();

        let keys = vec!["key", "key2", "key3"];
        for key in &keys {
            assert!(mem.insert(key, "loco").await.is_ok());
        }
        for key in &keys {
            assert!(mem.contains_key(key).await.is_ok());
        }
        assert!(mem.clear().await.is_ok());
        for key in &keys {
            assert!(!mem.contains_key(key).await.unwrap());
        }
    }
}
