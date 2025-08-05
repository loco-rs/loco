//! # Cache Module
//!
//! This module provides a generic cache interface for various cache drivers.
pub mod drivers;

use std::{future::Future, time::Duration};

use serde::{de::DeserializeOwned, Serialize};

pub use self::drivers::CacheDriver;
use crate::config;
use crate::Result as LocoResult;
use std::sync::Arc;

/// Errors related to cache operations
#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum CacheError {
    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[cfg(feature = "cache_redis")]
    #[error(transparent)]
    Redis(#[from] bb8_redis::redis::RedisError),

    #[cfg(feature = "cache_redis")]
    #[error(transparent)]
    RedisConnectionError(#[from] bb8_redis::bb8::RunError<bb8_redis::redis::RedisError>),
}

pub type CacheResult<T> = std::result::Result<T, CacheError>;

/// Create a provider
///
/// # Errors
///
/// This function will return an error if fails to build
#[allow(clippy::unused_async)]
pub async fn create_cache_provider(config: &config::Config) -> crate::Result<Arc<Cache>> {
    match &config.cache {
        #[cfg(feature = "cache_redis")]
        config::CacheConfig::Redis(config) => {
            let cache = crate::cache::drivers::redis::new(config).await?;
            Ok(Arc::new(cache))
        }
        #[cfg(feature = "cache_inmem")]
        config::CacheConfig::InMem(config) => {
            let cache = crate::cache::drivers::inmem::new(config);
            Ok(Arc::new(cache))
        }
        config::CacheConfig::Null => {
            let driver = crate::cache::drivers::null::new();
            Ok(Arc::new(Cache::new(driver)))
        }
    }
}

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

    /// Pings the cache to check if it is reachable.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    ///
    /// pub async fn ping() -> CacheResult<()> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
    ///     cache.ping().await
    /// }
    /// ```
    ///
    /// # Errors
    /// A [`CacheResult`] indicating whether the cache is reachable.
    pub async fn ping(&self) -> CacheResult<()> {
        self.driver.ping().await
    }

    /// Checks if a key exists in the cache.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    ///
    /// pub async fn contains_key() -> CacheResult<bool> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
    ///     cache.contains_key("key").await
    /// }
    /// ```
    ///
    /// # Errors
    /// A [`CacheResult`] indicating whether the key exists in the cache.
    pub async fn contains_key(&self, key: &str) -> CacheResult<bool> {
        self.driver.contains_key(key).await
    }

    /// Retrieves a value from the cache based on the provided key and deserializes it.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct User {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// pub async fn get_user() -> CacheResult<Option<User>> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
    ///     cache.get::<User>("user:1").await
    /// }
    /// ```
    ///
    /// # Example with String
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    ///
    /// pub async fn get_string() -> CacheResult<Option<String>> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
    ///     cache.get::<String>("key").await
    /// }
    /// ```
    ///
    /// # Errors
    /// A [`CacheResult`] containing an `Option` representing the retrieved
    /// and deserialized value.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> CacheResult<Option<T>> {
        let result = self.driver.get(key).await?;
        if let Some(value) = result {
            let deserialized = serde_json::from_str::<T>(&value)
                .map_err(|e| CacheError::Deserialization(e.to_string()))?;
            Ok(Some(deserialized))
        } else {
            Ok(None)
        }
    }

    /// Inserts a serializable value into the cache with the provided key.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct User {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// pub async fn insert() -> CacheResult<()> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
    ///     let user = User { name: "Alice".to_string(), age: 30 };
    ///     cache.insert("user:1", &user).await
    /// }
    /// ```
    ///
    /// # Example with String
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    ///
    /// pub async fn insert_string() -> CacheResult<()> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
    ///     cache.insert("key", &"value".to_string()).await
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A [`CacheResult`] indicating the success of the operation.
    pub async fn insert<T: Serialize + Sync + ?Sized>(
        &self,
        key: &str,
        value: &T,
    ) -> CacheResult<()> {
        let serialized =
            serde_json::to_string(value).map_err(|e| CacheError::Serialization(e.to_string()))?;
        self.driver.insert(key, &serialized).await
    }

    /// Inserts a serializable value into the cache with the provided key and expiry duration.
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct User {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// pub async fn insert() -> CacheResult<()> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
    ///     let user = User { name: "Alice".to_string(), age: 30 };
    ///     cache.insert_with_expiry("user:1", &user, Duration::from_secs(300)).await
    /// }
    /// ```
    ///
    /// # Example with String
    /// ```
    /// use std::time::Duration;
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    ///
    /// pub async fn insert_string() -> CacheResult<()> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
    ///     cache.insert_with_expiry("key", &"value".to_string(), Duration::from_secs(300)).await
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A [`CacheResult`] indicating the success of the operation.
    pub async fn insert_with_expiry<T: Serialize + Sync + ?Sized>(
        &self,
        key: &str,
        value: &T,
        duration: Duration,
    ) -> CacheResult<()> {
        let serialized =
            serde_json::to_string(value).map_err(|e| CacheError::Serialization(e.to_string()))?;
        self.driver
            .insert_with_expiry(key, &serialized, duration)
            .await
    }

    /// Retrieves and deserializes the value associated with the given key from the cache,
    /// or inserts it if it does not exist, using the provided closure to
    /// generate the value.
    ///
    /// # Example
    /// ```
    /// use loco_rs::{app::AppContext};
    /// use loco_rs::tests_cfg::app::*;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, PartialEq, Debug)]
    /// struct User {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// pub async fn get_or_insert(){
    ///    let app_ctx = get_app_context().await;
    ///    let user = app_ctx.cache.get_or_insert::<User, _>("user:1", async {
    ///            Ok(User { name: "Alice".to_string(), age: 30 })
    ///     }).await.unwrap();
    ///    assert_eq!(user.name, "Alice");
    /// }
    /// ```
    ///
    /// # Example with String
    /// ```
    /// use loco_rs::{app::AppContext};
    /// use loco_rs::tests_cfg::app::*;
    ///
    /// pub async fn get_or_insert_string(){
    ///    let app_ctx = get_app_context().await;
    ///    let res = app_ctx.cache.get_or_insert::<String, _>("key", async {
    ///            Ok("value".to_string())
    ///     }).await.unwrap();
    ///    assert_eq!(res, "value");
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A [`LocoResult`] indicating the success of the operation.
    pub async fn get_or_insert<T, F>(&self, key: &str, f: F) -> LocoResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: Future<Output = LocoResult<T>> + Send,
    {
        if let Some(value) = self.get::<T>(key).await? {
            Ok(value)
        } else {
            let value = f.await?;
            self.insert(key, &value).await?;
            Ok(value)
        }
    }

    /// Retrieves and deserializes the value associated with the given key from the cache,
    /// or inserts it (with expiry after provided duration) if it does not
    /// exist, using the provided closure to generate the value.
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    /// use loco_rs::{app::AppContext};
    /// use loco_rs::tests_cfg::app::*;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, PartialEq, Debug)]
    /// struct User {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// pub async fn get_or_insert(){
    ///    let app_ctx = get_app_context().await;
    ///    let user = app_ctx.cache.get_or_insert_with_expiry::<User, _>("user:1", Duration::from_secs(300), async {
    ///            Ok(User { name: "Alice".to_string(), age: 30 })
    ///     }).await.unwrap();
    ///    assert_eq!(user.name, "Alice");
    /// }
    /// ```
    ///
    /// # Example with String
    /// ```
    /// use std::time::Duration;
    /// use loco_rs::{app::AppContext};
    /// use loco_rs::tests_cfg::app::*;
    ///
    /// pub async fn get_or_insert_string(){
    ///    let app_ctx = get_app_context().await;
    ///    let res = app_ctx.cache.get_or_insert_with_expiry::<String, _>("key", Duration::from_secs(300), async {
    ///            Ok("value".to_string())
    ///     }).await.unwrap();
    ///    assert_eq!(res, "value");
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A [`LocoResult`] indicating the success of the operation.
    pub async fn get_or_insert_with_expiry<T, F>(
        &self,
        key: &str,
        duration: Duration,
        f: F,
    ) -> LocoResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: Future<Output = LocoResult<T>> + Send,
    {
        if let Some(value) = self.get::<T>(key).await? {
            Ok(value)
        } else {
            let value = f.await?;
            self.insert_with_expiry(key, &value, duration).await?;
            Ok(value)
        }
    }

    /// Removes a key-value pair from the cache.
    ///
    /// # Example
    /// ```
    /// use loco_rs::cache::{self, CacheResult};
    /// use loco_rs::config::InMemCacheConfig;
    ///
    /// pub async fn remove() -> CacheResult<()> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
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
    /// use loco_rs::config::InMemCacheConfig;
    ///
    /// pub async fn clear() -> CacheResult<()> {
    ///     let config = InMemCacheConfig { max_capacity: 100 };
    ///     let cache = cache::Cache::new(cache::drivers::inmem::new(&config).driver);
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
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn can_get_or_insert() {
        let app_ctx = tests_cfg::app::get_app_context().await;
        let get_key = "loco";

        assert_eq!(app_ctx.cache.get::<String>(get_key).await.unwrap(), None);

        let result = app_ctx
            .cache
            .get_or_insert::<String, _>(get_key, async { Ok("loco-cache-value".to_string()) })
            .await
            .unwrap();

        assert_eq!(result, "loco-cache-value".to_string());
        assert_eq!(
            app_ctx.cache.get::<String>(get_key).await.unwrap(),
            Some("loco-cache-value".to_string())
        );
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestUser {
        name: String,
        age: u32,
    }

    #[tokio::test]
    async fn can_serialize_deserialize() {
        let app_ctx = tests_cfg::app::get_app_context().await;
        let key = "user:test";

        // Test user data
        let user = TestUser {
            name: "Test User".to_string(),
            age: 42,
        };

        // Insert serialized user
        app_ctx.cache.insert(key, &user).await.unwrap();

        // Retrieve and deserialize user
        let retrieved: Option<TestUser> = app_ctx.cache.get(key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), user);
    }

    #[tokio::test]
    async fn can_get_or_insert_generic() {
        let app_ctx = tests_cfg::app::get_app_context().await;
        let key = "user:get_or_insert";

        // The key should not exist initially
        let no_user: Option<TestUser> = app_ctx.cache.get(key).await.unwrap();
        assert!(no_user.is_none());

        // Get or insert should create the user
        let user = app_ctx
            .cache
            .get_or_insert::<TestUser, _>(key, async {
                Ok(TestUser {
                    name: "Alice".to_string(),
                    age: 30,
                })
            })
            .await
            .unwrap();

        assert_eq!(user.name, "Alice");
        assert_eq!(user.age, 30);

        // Verify the user was stored in the cache
        let retrieved: TestUser = app_ctx
            .cache
            .get_or_insert::<TestUser, _>(key, async {
                // This should not be called
                Ok(TestUser {
                    name: "Bob".to_string(),
                    age: 25,
                })
            })
            .await
            .unwrap();

        // Should retrieve Alice, not Bob
        assert_eq!(retrieved.name, "Alice");
        assert_eq!(retrieved.age, 30);
    }
}
