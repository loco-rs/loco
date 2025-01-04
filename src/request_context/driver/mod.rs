pub mod cookie;

use std::sync::Arc;

use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use tower_sessions::Session;

use crate::request_context::driver::cookie::CookieMap;

#[derive(Debug, Clone)]
pub enum Driver {
    TowerSession(Session),
    CookieMap(Arc<Mutex<CookieMap>>),
}

impl Driver {
    /// Inserts a `impl Serialize` value into the session.
    /// # Arguments
    /// * `key` - The key to store the value
    /// * `value` - The value to store
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be serialized
    /// * `TowerSessionError` - When the value is unable to be serialized or if
    ///   the session has not been hydrated and loading from the store fails, we
    ///   fail with `Error::Store`
    pub async fn insert<T>(&mut self, key: &str, value: T) -> Result<(), DriverError>
    where
        T: serde::Serialize + Send + Sync,
    {
        match self {
            Self::CookieMap(cookie_map) => {
                cookie_map.lock().await.insert(key, value)?;
            }
            Self::TowerSession(session) => {
                session.insert(key, value).await?;
            }
        }
        Ok(())
    }

    /// Gets a `impl DeserializeOwned` value from the session.
    /// # Arguments
    /// * `key` - The key to get the value from
    /// # Returns
    /// * `Option<T>` - The value if it exists
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be deserialized
    /// * `TowerSessionError` - When the value is unable to be deserialized or
    ///   if the session has not been hydrated and loading from the store fails,
    ///   we fail with `Error::Store`
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, DriverError> {
        match self {
            Self::CookieMap(cookie_map) => Ok(cookie_map.lock().await.get(key)?),
            Self::TowerSession(session) => Ok(session.get(key).await?),
        }
    }

    /// Removes a `serde_json::Value` from the session.
    ///
    /// # Arguments
    /// * `key` - The key to remove from the session
    ///
    /// # Return
    /// * `Option<T>` - The value if it exists
    ///
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be deserialized
    /// * `TowerSessionError` - When the value is unable to be deserialized or
    ///   if the session has not been hydrated and loading from the store fails,
    ///   we fail with `Error::Store`
    pub async fn remove<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, DriverError> {
        match self {
            Self::CookieMap(cookie_map) => Ok(cookie_map.lock().await.remove(key)?),
            Self::TowerSession(session) => Ok(session.remove(key).await?),
        }
    }

    /// Tower - Clears the session but not the session store.
    /// Cookie - Clear the session map.
    pub async fn clear(&mut self) {
        match self {
            Self::CookieMap(cookie_map) => {
                cookie_map.lock().await.clear();
            }
            Self::TowerSession(session) => {
                session.clear().await;
            }
        }
    }

    /// Tower - Flush the session store.
    /// Cookie - Clear the session map.
    ///
    /// # Returns
    /// * `()`
    ///
    /// # Errors
    /// [`DriverError`] - When the session store fails to flush
    pub async fn flush(&mut self) -> Result<(), DriverError> {
        match self {
            Self::CookieMap(cookie_map) => {
                cookie_map.lock().await.clear();
                Ok(())
            }
            Self::TowerSession(session) => Ok(session.flush().await?),
        }
    }

    /// Check if the key exists in the session.
    /// # Arguments
    /// * `key` - The key to check
    /// # Returns
    /// * `bool` - True if the key exists, otherwise false
    /// # Errors
    /// * `DriverError` - If there is an error while checking the key
    pub async fn exists(&self, key: &str) -> Result<bool, DriverError> {
        match self {
            Self::CookieMap(cookie_map) => Ok(cookie_map.lock().await.exists(key)),
            Self::TowerSession(session) => Ok(session.get_value(key).await?.is_some()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("CookieMapError: {0}")]
    CookieMapError(#[from] cookie::CookieMapError),
    #[error("TowerSessionError: {0}")]
    TowerSessionError(#[from] tower_sessions::session::Error),
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, sync::Arc};

    use tower_sessions::{MemoryStore, Session};

    use super::*;

    fn create_session() -> Session {
        let store = Arc::new(MemoryStore::default());
        Session::new(None, store, None)
    }

    #[tokio::test]
    async fn test_driver_insert() {
        let hash_map = HashMap::new();
        let mut driver = Driver::CookieMap(Arc::new(Mutex::new(CookieMap::new(hash_map))));
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_driver_insert_tower_session() {
        let session = create_session();
        let mut driver = Driver::TowerSession(session);
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_driver_get() {
        let hash_map = HashMap::new();
        let mut driver = Driver::CookieMap(Arc::new(Mutex::new(CookieMap::new(hash_map))));
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_driver_get_tower_session() {
        let session = create_session();
        let mut driver = Driver::TowerSession(session);
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_driver_remove() {
        let hash_map = HashMap::new();
        let mut driver = Driver::CookieMap(Arc::new(Mutex::new(CookieMap::new(hash_map))));
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
        let removed_value: Option<String> =
            driver.remove("test").await.expect("Failed to remove value");
        assert_eq!(removed_value, Some("test".to_string()));
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_driver_remove_tower_session() {
        let session = create_session();
        let mut driver = Driver::TowerSession(session);
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
        let removed_value: Option<String> =
            driver.remove("test").await.expect("Failed to remove value");
        assert_eq!(removed_value, Some("test".to_string()));
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_driver_clear() {
        let hash_map = HashMap::new();
        let mut driver = Driver::CookieMap(Arc::new(Mutex::new(CookieMap::new(hash_map))));
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
        driver.clear().await;
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_driver_clear_tower_session() {
        let session = create_session();
        let mut driver = Driver::TowerSession(session);
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
        driver.clear().await;
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_driver_flush() -> Result<(), DriverError> {
        let hash_map = HashMap::new();
        let mut driver = Driver::CookieMap(Arc::new(Mutex::new(CookieMap::new(hash_map))));
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
        driver.flush().await?;
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_driver_flush_tower_session() -> Result<(), DriverError> {
        let session = create_session();
        let mut driver = Driver::TowerSession(session);
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, Some("test".to_string()));
        driver.flush().await?;
        let value: Option<String> = driver.get("test").await.expect("Failed to get value");
        assert_eq!(value, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_driver_value_exist() -> Result<(), DriverError> {
        let hash_map = HashMap::new();
        let mut driver = Driver::CookieMap(Arc::new(Mutex::new(CookieMap::new(hash_map))));
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        assert!(driver.exists("test").await?);
        assert!(!driver.exists("test2").await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_driver_value_exist_tower_session() -> Result<(), DriverError> {
        let session = create_session();
        let mut driver = Driver::TowerSession(session);
        driver
            .insert("test", "test")
            .await
            .expect("Failed to insert value");
        assert!(driver.exists("test").await?);
        assert!(!driver.exists("test2").await?);
        Ok(())
    }
}
