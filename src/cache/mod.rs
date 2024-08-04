//! # Cache Module
//!
//! This module provides a generic cache interface for various cache drivers.
pub mod drivers;

use std::future::Future;

use self::drivers::CacheDriver;

/// Errors related to cache operations
#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum CacheError {
    #[error(transparent)]
    Model(#[from] crate::model::ModelError),

    #[error(transparent)]
    Loco(#[from] crate::Error),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

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

    /// Retrieves the value associated with the given key from the cache,
    /// or inserts it if it does not exist, using the provided closure to generate the value.
    ///
    /// # Example
    /// ```
    /// use loco_rs::{app::AppContext, cache::{self, CacheResult}};
    /// use loco_rs::tests_cfg::app::*;
    ///
    /// pub async fn get_or_insert(){
    ///    let app_ctx = get_app_context().await;
    ///    let res = app_ctx.cache.get_or_insert("key", async {
    ///            Ok("value".to_string())
    ///     }).await.unwrap();
    ///    assert_eq!(res, "value");
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A [`CacheResult`] indicating the success of the operation.
    pub async fn get_or_insert<F>(&self, key: &str, f: F) -> CacheResult<String>
    where
        F: Future<Output = CacheResult<String>> + Send,
    {
        if let Some(value) = self.driver.get(key).await? {
            Ok(value)
        } else {
            let value = f.await?;
            self.driver.insert(key, &value).await?;
            Ok(value)
        }
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

#[cfg(test)]
mod tests {

    use crate::tests_cfg;

    #[tokio::test]
    async fn can_get_or_insert() {
        let app_ctx = tests_cfg::app::get_app_context().await;
        let get_key = "loco";

        assert_eq!(app_ctx.cache.get(get_key).await.unwrap(), None);

        let result = app_ctx
            .cache
            .get_or_insert(get_key, async { Ok("loco-cache-value".to_string()) })
            .await
            .unwrap();

        assert_eq!(result, "loco-cache-value".to_string());
        assert_eq!(
            app_ctx.cache.get(get_key).await.unwrap(),
            Some("loco-cache-value".to_string())
        );
    }
}
