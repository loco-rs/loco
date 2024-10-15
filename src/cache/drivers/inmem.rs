//! # In-Memory Cache Driver
//!
//! This module implements a cache driver using an in-memory cache.
use std::sync::Arc;

use async_trait::async_trait;
use moka::sync::Cache;

use super::CacheDriver;
use crate::cache::CacheResult;

/// Creates a new instance of the in-memory cache driver, with a default Loco
/// configuration.
///
/// # Returns
///
/// A boxed [`CacheDriver`] instance.
#[must_use]
pub fn new() -> Box<dyn CacheDriver<Key = String, Value = String>> {
    let cache = Cache::builder().max_capacity(32 * 1024 * 1024).build();
    Inmem::from(cache)
}

/// Represents the in-memory cache driver.
#[derive(Debug)]
pub struct Inmem {
    cache: Cache<String, String>,
}

impl Inmem {
    /// Constructs a new [`Inmem`] instance from a given cache.
    ///
    /// # Returns
    ///
    /// A boxed [`CacheDriver`] instance.
    #[must_use]
    pub fn from(
        cache: Cache<String, String>,
    ) -> Box<dyn CacheDriver<Key = String, Value = String>> {
        Box::new(Self { cache })
    }
}

#[async_trait]
impl CacheDriver for Inmem {
    /// The type used for cache keys. Must be serializable and deserializable.
    type Key = String;

    /// The type used for cache values. Must be serializable and deserializable.
    type Value = String;

    /// Checks if a key exists in the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn contains_key(&self, key: &Self::Key) -> CacheResult<bool> {
        Ok(self.cache.contains_key(key))
    }

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn get(&self, key: &Self::Key) -> CacheResult<Option<Self::Value>> {
        Ok(self.cache.get(key))
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn insert(&self, key: &Self::Key, value: &Self::Value) -> CacheResult<()> {
        self.cache
            .insert(key.to_string(), Arc::new(value).to_string());
        Ok(())
    }

    /// Removes a key-value pair from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn remove(&self, key: &Self::Key) -> CacheResult<()> {
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

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn is_contains_key() {
        let mem = new();
        let key = "key".to_string();
        let value = "loco".to_string();
        assert!(!mem.contains_key(&key).await.unwrap());
        assert!(mem.insert(&key, &value).await.is_ok());
        assert!(mem.contains_key(&key).await.unwrap());
    }

    #[tokio::test]
    async fn can_get_key_value() {
        let mem = new();
        let key = "key".to_string();
        let value = "loco".to_string();
        assert!(mem.insert(&key, &value).await.is_ok());
        assert_eq!(mem.get(&key).await.unwrap(), Some(value));

        //try getting key that not exists
        let not_found_key = "not-found".to_string();
        assert_eq!(mem.get(&not_found_key).await.unwrap(), None);
    }

    #[tokio::test]
    async fn can_remove_key() {
        let mem = new();
        let key = "key".to_string();
        let value = "loco".to_string();
        assert!(mem.insert(&key, &value).await.is_ok());
        assert!(mem.contains_key(&key).await.unwrap());
        mem.remove(&key).await.unwrap();
        assert!(!mem.contains_key(&key).await.unwrap());
    }

    #[tokio::test]
    async fn can_clear() {
        let mem = new();
        let keys = vec!["key".to_string(), "key2".to_string(), "key3".to_string()];
        for key in &keys {
            assert!(mem.insert(key, &"loco".to_string()).await.is_ok());
        }
        for key in &keys {
            assert!(mem.contains_key(key).await.unwrap());
        }
        assert!(mem.clear().await.is_ok());
        for key in &keys {
            assert!(!mem.contains_key(key).await.unwrap());
        }
    }
}
