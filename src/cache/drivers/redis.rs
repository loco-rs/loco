//! # Redis Cache Driver
//!
//! This module implements a cache driver using Redis.
use std::time::Duration;

use async_trait::async_trait;
use bb8::Pool;
use bb8_redis::{
    bb8,
    redis::{cmd, AsyncCommands},
    RedisConnectionManager,
};

use super::CacheDriver;
use crate::cache::CacheResult;
use crate::config::RedisCacheConfig;

/// Creates a new instance of the Redis cache driver with a default configuration.
///
/// # Returns
///
/// A [`Cache`] instance.
///
/// # Errors
///
/// Returns a `CacheError` if there is an error connecting to Redis.
pub async fn new(config: &RedisCacheConfig) -> CacheResult<crate::cache::Cache> {
    let manager = RedisConnectionManager::new(config.uri.clone())?;
    let pool = Pool::builder()
        .max_size(config.max_size)
        .build(manager)
        .await?;

    Ok(crate::cache::Cache::new(Redis::from(pool)))
}

/// Represents the Redis cache driver.
#[derive(Clone, Debug)]
pub struct Redis {
    pool: Pool<RedisConnectionManager>,
}

impl Redis {
    /// Constructs a new [`Redis`] instance from a given connection pool.
    ///
    /// # Returns
    ///
    /// A boxed [`CacheDriver`] instance.
    #[must_use]
    pub fn from(pool: Pool<RedisConnectionManager>) -> Box<dyn CacheDriver> {
        Box::new(Self { pool })
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
        let mut connection = self.pool.get().await?;
        Ok(connection.exists(key).await?)
    }

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn get(&self, key: &str) -> CacheResult<Option<String>> {
        let mut conn = self.pool.get().await?;
        let result: Option<String> = conn.get(key).await?;
        Ok(result)
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn insert(&self, key: &str, value: &str) -> CacheResult<()> {
        let mut conn = self.pool.get().await?;
        conn.set::<_, _, ()>(key, value).await?;
        Ok(())
    }

    /// Inserts a key-value pair into the cache that expires after the specified
    /// number of seconds.
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
    ) -> CacheResult<()> {
        let mut conn = self.pool.get().await?;
        // Redis expects the expiry in seconds as a u64
        conn.set_ex::<_, _, ()>(key, value, duration.as_secs())
            .await?;
        Ok(())
    }

    /// Removes a key-value pair from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn remove(&self, key: &str) -> CacheResult<()> {
        let mut conn = self.pool.get().await?;
        conn.del::<_, ()>(key).await?;
        Ok(())
    }

    /// Clears all key-value pairs from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn clear(&self) -> CacheResult<()> {
        let mut conn = self.pool.get().await?;
        cmd("FLUSHDB").query_async::<()>(&mut *conn).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use testcontainers::{
        core::{ContainerPort, WaitFor},
        runners::AsyncRunner,
        ContainerAsync, GenericImage,
    };

    use super::*;

    async fn setup_redis_driver() -> (Box<dyn CacheDriver>, ContainerAsync<GenericImage>) {
        let redis_image = GenericImage::new("redis", "7")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

        let container = redis_image
            .start()
            .await
            .expect("Failed to start Redis container");
        let host_port = container
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get host port");
        let redis_url = format!("redis://127.0.0.1:{}", host_port);

        let redis_config = crate::config::RedisCacheConfig {
            uri: redis_url,
            max_size: 10,
        };

        let cache = new(&redis_config)
            .await
            .expect("Failed to create Redis driver");

        // Extract the driver from the Cache
        let driver = cache.driver;

        (driver, container)
    }

    #[tokio::test]
    async fn test_contains_key() {
        let (redis, _container) = setup_redis_driver().await;

        assert!(!redis
            .contains_key("test_key")
            .await
            .expect("Failed to check if key exists"));

        redis
            .insert("test_key", "test_value")
            .await
            .expect("Failed to insert key");

        assert!(redis
            .contains_key("test_key")
            .await
            .expect("Failed to check if key exists after insertion"));
    }

    #[tokio::test]
    async fn test_get_key_value() {
        let (redis, _container) = setup_redis_driver().await;

        redis
            .insert("test_key", "test_value")
            .await
            .expect("Failed to insert key");

        assert_eq!(
            redis
                .get("test_key")
                .await
                .expect("Failed to get value for key"),
            Some("test_value".to_string())
        );

        assert_eq!(
            redis
                .get("non_existent_key")
                .await
                .expect("Failed to get value for non-existent key"),
            None
        );
    }

    #[tokio::test]
    async fn test_remove_key() {
        let (redis, _container) = setup_redis_driver().await;

        redis
            .insert("test_key", "test_value")
            .await
            .expect("Failed to insert key");

        assert!(redis
            .contains_key("test_key")
            .await
            .expect("Failed to check if key exists"));

        redis
            .remove("test_key")
            .await
            .expect("Failed to remove key");

        assert!(!redis
            .contains_key("test_key")
            .await
            .expect("Failed to check if key exists after removal"));
    }

    #[tokio::test]
    async fn test_clear() {
        let (redis, _container) = setup_redis_driver().await;

        let keys = vec!["key1", "key2", "key3"];
        for key in &keys {
            redis
                .insert(key, "test_value")
                .await
                .expect("Failed to insert key");
        }

        for key in &keys {
            assert!(redis
                .contains_key(key)
                .await
                .expect("Failed to check if key exists"));
        }

        redis.clear().await.expect("Failed to clear cache");

        for key in &keys {
            assert!(!redis
                .contains_key(key)
                .await
                .expect("Failed to check if key exists after clear"));
        }
    }

    #[tokio::test]
    async fn test_expiry() {
        let (redis, _container) = setup_redis_driver().await;

        redis
            .insert_with_expiry("expiring_key", "test_value", Duration::from_secs(1))
            .await
            .expect("Failed to insert key with expiry");

        assert!(redis
            .contains_key("expiring_key")
            .await
            .expect("Failed to check if key exists"));

        tokio::time::sleep(Duration::from_secs(2)).await;

        assert!(!redis
            .contains_key("expiring_key")
            .await
            .expect("Failed to check if key exists after expiry"));
    }
}
