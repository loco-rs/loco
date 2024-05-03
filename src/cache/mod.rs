//! # Cache Module
//!
//! This module provides a generic cache interface for various cache drivers.
pub mod drivers;

use self::drivers::CacheDriver;

/// Errors related to cache operations
#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum CacheError {}

pub type CacheResult<T> = std::result::Result<T, CacheError>;

/// Represents a cache instance
pub struct Cache {
    /// The cache driver used for underlying operations
    pub driver: Box<dyn CacheDriver>,
}

impl Cache {
    /// Creates a new cache instance with the specified cache driver.
    #[must_use]
    pub fn new(driver: Box<dyn CacheDriver>) -> Self {
        Self { driver }
    }

    /// Checks if a key exists in the cache.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    ///
    /// pub async fn contains_key() -> CacheResult<bool> {
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new());
    ///     cache.contains_key("key").await
    /// }
    /// ```
    ///
    /// # Errors
    /// A [`CacheResult`] indicating whether the key exists in the cache.
    pub async fn contains_key(&self, key: &str) -> CacheResult<bool> {
        self.driver.contains_key(key).await
    }

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    ///
    /// pub async fn get_key() -> CacheResult<Option<String>> {
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new());
    ///     cache.get("key").await
    /// }
    /// ```
    ///
    /// # Errors
    /// A [`CacheResult`] containing an `Option` representing the retrieved
    /// value.
    pub async fn get(&self, key: &str) -> CacheResult<Option<String>> {
        self.driver.get(key).await
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    ///
    /// pub async fn insert() -> CacheResult<()> {
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new());
    ///     cache.insert("key", "value").await
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A [`CacheResult`] indicating the success of the operation.
    pub async fn insert(&self, key: &str, value: &str) -> CacheResult<()> {
        self.driver.insert(key, value).await
    }

    /// Removes a key-value pair from the cache.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    ///
    /// pub async fn remove() -> CacheResult<()> {
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new());
    ///     cache.remove("key").await
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A [`CacheResult`] indicating the success of the operation.
    pub async fn remove(&self, key: &str) -> CacheResult<()> {
        self.driver.remove(key).await
    }

    /// Clears all key-value pairs from the cache.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    ///
    /// pub async fn clear() -> CacheResult<()> {
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new());
    ///     cache.clear().await
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A [`CacheResult`] indicating the success of the operation.
    pub async fn clear(&self) -> CacheResult<()> {
        self.driver.clear().await
    }
}
