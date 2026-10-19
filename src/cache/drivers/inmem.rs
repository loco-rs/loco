//! # In-Memory Cache Driver
//!
//! This module implements a cache driver that keeps entries in a map inside
//! the process. Entries can expire, and once the cache holds `max_capacity`
//! entries it evicts the least recently used one to make room.
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use async_trait::async_trait;

use super::CacheDriver;
use crate::cache::CacheResult;
use crate::config::InMemCacheConfig;

/// Creates a new instance of the in-memory cache driver, with a default Loco
/// configuration.
///
/// # Returns
///
/// A [`crate::cache::Cache`] instance.
#[must_use]
pub fn new(config: &InMemCacheConfig) -> crate::cache::Cache {
    crate::cache::Cache::new(Box::new(Inmem::new(config.max_capacity)))
}

/// Represents the in-memory cache driver.
#[derive(Debug)]
pub struct Inmem {
    store: Mutex<Store>,
    max_capacity: u64,
}

#[derive(Debug, Default)]
struct Store {
    entries: HashMap<String, Entry>,
    /// Keys by when they were last used, oldest first.
    by_use: BTreeMap<u64, String>,
    clock: u64,
}

#[derive(Debug)]
struct Entry {
    value: String,
    expires_at: Option<Instant>,
    used: u64,
}

impl Inmem {
    /// Constructs an empty cache that holds at most `max_capacity` entries.
    #[must_use]
    pub fn new(max_capacity: u64) -> Self {
        Self {
            store: Mutex::default(),
            max_capacity,
        }
    }

    fn with_store<T>(&self, f: impl FnOnce(&mut Store) -> T) -> T {
        f(&mut self.store.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl Store {
    /// The live value for `key`, marked as just used. An expired entry is
    /// removed.
    fn get(&mut self, key: &str) -> Option<&String> {
        let expired = self
            .entries
            .get(key)?
            .expires_at
            .is_some_and(|at| at <= Instant::now());
        if expired {
            self.remove(key);
            return None;
        }
        self.clock += 1;
        let entry = self.entries.get_mut(key)?;
        let key = self.by_use.remove(&entry.used)?;
        entry.used = self.clock;
        self.by_use.insert(self.clock, key);
        Some(&entry.value)
    }

    fn insert(&mut self, key: &str, value: &str, ttl: Option<Duration>, max_capacity: u64) {
        self.remove(key);
        if max_capacity == 0 {
            return;
        }
        while self.entries.len() as u64 >= max_capacity {
            let Some((_, oldest)) = self.by_use.pop_first() else {
                break;
            };
            self.entries.remove(&oldest);
        }
        self.clock += 1;
        self.by_use.insert(self.clock, key.to_string());
        self.entries.insert(
            key.to_string(),
            Entry {
                value: value.to_string(),
                expires_at: ttl.and_then(|ttl| Instant::now().checked_add(ttl)),
                used: self.clock,
            },
        );
    }

    fn remove(&mut self, key: &str) {
        if let Some(entry) = self.entries.remove(key) {
            self.by_use.remove(&entry.used);
        }
    }
}

#[async_trait]
impl CacheDriver for Inmem {
    /// Pings the cache to check if it is reachable.
    ///
    /// # Errors
    ///
    /// Returns always error
    async fn ping(&self) -> CacheResult<()> {
        Ok(())
    }

    /// Checks if a key exists in the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn contains_key(&self, key: &str) -> CacheResult<bool> {
        Ok(self.with_store(|store| store.get(key).is_some()))
    }

    /// Retrieves a value from the cache based on the provided key.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn get(&self, key: &str) -> CacheResult<Option<String>> {
        Ok(self.with_store(|store| store.get(key).cloned()))
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn insert(&self, key: &str, value: &str) -> CacheResult<()> {
        self.with_store(|store| store.insert(key, value, None, self.max_capacity));
        Ok(())
    }

    /// Inserts a key-value pair into the cache that expires after the specified
    /// number of seconds.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::cache::CacheError`] if there is an error during the
    /// operation.
    async fn insert_with_expiry(
        &self,
        key: &str,
        value: &str,
        duration: Duration,
    ) -> CacheResult<()> {
        self.with_store(|store| store.insert(key, value, Some(duration), self.max_capacity));
        Ok(())
    }

    /// Removes a key-value pair from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn remove(&self, key: &str) -> CacheResult<()> {
        self.with_store(|store| store.remove(key));
        Ok(())
    }

    /// Clears all key-value pairs from the cache.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if there is an error during the operation.
    async fn clear(&self) -> CacheResult<()> {
        self.with_store(|store| *store = Store::default());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InMemCacheConfig;

    fn create_test_config() -> InMemCacheConfig {
        InMemCacheConfig { max_capacity: 100 }
    }

    #[tokio::test]
    async fn ping_returns_pong_when_cache_is_accessible() {
        let config = create_test_config();
        let mem = new(&config);
        assert!(mem.ping().await.is_ok());
    }

    #[tokio::test]
    async fn is_contains_key() {
        let config = create_test_config();
        let mem = new(&config);
        assert!(!mem.contains_key("key").await.unwrap());
        assert!(mem.insert("key", "loco").await.is_ok());
        assert!(mem.contains_key("key").await.unwrap());
    }

    #[tokio::test]
    async fn can_get_key_value() {
        let config = create_test_config();
        let mem = new(&config);
        assert!(mem.insert("key", "loco").await.is_ok());
        assert_eq!(
            mem.get::<String>("key").await.unwrap(),
            Some("loco".to_string())
        );

        //try getting key that not exists
        assert_eq!(mem.get::<String>("not-found").await.unwrap(), None);
    }

    #[tokio::test]
    async fn can_remove_key() {
        let config = create_test_config();
        let mem = new(&config);
        assert!(mem.insert("key", "loco").await.is_ok());
        assert!(mem.contains_key("key").await.unwrap());
        mem.remove("key").await.unwrap();
        assert!(!mem.contains_key("key").await.unwrap());
    }

    #[tokio::test]
    async fn can_clear() {
        let config = create_test_config();
        let mem = new(&config);

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

    #[tokio::test]
    async fn entries_expire() {
        let mem = Inmem::new(10);
        mem.insert_with_expiry("short", "a", Duration::from_millis(20))
            .await
            .unwrap();
        mem.insert_with_expiry("long", "b", Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(mem.get("short").await.unwrap().as_deref(), Some("a"));
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(mem.get("short").await.unwrap(), None);
        assert!(!mem.contains_key("short").await.unwrap());
        assert_eq!(mem.get("long").await.unwrap().as_deref(), Some("b"));
        // A plain insert over an expiring entry keeps it for good.
        mem.insert("long", "c").await.unwrap();
        assert!(mem.with_store(|store| store.entries["long"].expires_at.is_none()));
    }

    #[tokio::test]
    async fn evicts_the_least_recently_used_entry_when_full() {
        let mem = Inmem::new(2);
        mem.insert("a", "1").await.unwrap();
        mem.insert("b", "2").await.unwrap();
        // Reading `a` makes `b` the oldest.
        mem.get("a").await.unwrap();
        mem.insert("c", "3").await.unwrap();
        assert!(mem.contains_key("a").await.unwrap());
        assert!(!mem.contains_key("b").await.unwrap());
        assert!(mem.contains_key("c").await.unwrap());
        // Overwriting a key does not evict another one.
        mem.insert("c", "4").await.unwrap();
        assert!(mem.contains_key("a").await.unwrap());
        assert_eq!(mem.get("c").await.unwrap().as_deref(), Some("4"));

        let none = Inmem::new(0);
        none.insert("a", "1").await.unwrap();
        assert!(!none.contains_key("a").await.unwrap());
    }
}
