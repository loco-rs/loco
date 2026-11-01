//! Ordered cache lookups and writes to every configured layer.
//!
//! Reads stop at the first hit. They do not populate earlier layers: the
//! [`CacheDriver`] interface does not expose remaining TTLs, so copying a hit
//! could accidentally keep it alive past its original expiry. Expiring writes
//! pass the same duration to each driver, using that driver's TTL precision.
//!
//! Mutations are not atomic across layers. Other processes' in-memory caches
//! are not invalidated; use finite TTLs when sharing Redis between processes.

use std::time::Duration;

use async_trait::async_trait;
use futures_util::future::{join_all, try_join_all};

use super::CacheDriver;
use crate::{
    cache::{Cache, CacheError, CacheResult},
    config::{CacheConfig, CacheErrorPolicy, MultiCacheConfig},
};

/// Creates a multi-layer cache from its configuration.
///
/// # Errors
/// Returns an error for an empty chain, nested `Multi` or `Null` layers, or
/// when any layer cannot be initialized. `error_policy` only applies after
/// initialization.
pub async fn new(config: &MultiCacheConfig) -> CacheResult<Cache> {
    // Validate the whole chain before opening any external connections.
    if config.layers.is_empty()
        || config
            .layers
            .iter()
            .any(|layer| matches!(layer, CacheConfig::Multi(_) | CacheConfig::Null))
    {
        return Err(CacheError::InvalidConfiguration(
            "Multi requires at least one layer; layers must be InMem or Redis".to_string(),
        ));
    }

    let layers = try_join_all(config.layers.iter().map(|layer| async {
        match layer {
            #[cfg(feature = "cache_inmem")]
            CacheConfig::InMem(config) => Ok(super::inmem::new(config).driver),
            #[cfg(feature = "cache_redis")]
            CacheConfig::Redis(config) => super::redis::new(config).await.map(|cache| cache.driver),
            // Already checked above; keep this exhaustive with no cache features.
            CacheConfig::Multi(_) | CacheConfig::Null => Err(CacheError::InvalidConfiguration(
                "Multi layers must be InMem or Redis".to_string(),
            )),
        }
    }))
    .await?;

    Ok(Cache::new(Box::new(Multi::new(
        layers,
        config.error_policy,
    )?)))
}

/// A cache driver that composes an ordered, non-empty list of drivers.
pub struct Multi {
    layers: Vec<Box<dyn CacheDriver>>,
    error_policy: CacheErrorPolicy,
}

impl Multi {
    /// Composes drivers in lookup order, including custom drivers.
    ///
    /// # Errors
    /// Returns an error if `layers` is empty.
    pub fn new(
        layers: Vec<Box<dyn CacheDriver>>,
        error_policy: CacheErrorPolicy,
    ) -> CacheResult<Self> {
        if layers.is_empty() {
            return Err(CacheError::InvalidConfiguration(
                "Multi requires at least one layer".to_string(),
            ));
        }
        Ok(Self {
            layers,
            error_policy,
        })
    }

    fn layer_error(index: usize, source: CacheError) -> CacheError {
        CacheError::Layer {
            index,
            source: Box::new(source),
        }
    }

    fn read_error(&self, operation: &str, index: usize, error: CacheError) -> CacheResult<()> {
        match self.error_policy {
            CacheErrorPolicy::Propagate => Err(Self::layer_error(index, error)),
            CacheErrorPolicy::Ignore => {
                tracing::warn!(layer = index, operation, %error, "cache_layer_error");
                Ok(())
            }
        }
    }

    fn check_results(
        operation: &str,
        results: Vec<CacheResult<()>>,
        policy: CacheErrorPolicy,
    ) -> CacheResult<()> {
        let mut first_error = None;
        for (index, result) in results.into_iter().enumerate() {
            if let Err(error) = result {
                tracing::warn!(layer = index, operation, %error, "cache_layer_error");
                if first_error.is_none() {
                    first_error = Some(Self::layer_error(index, error));
                }
            }
        }
        match (policy, first_error) {
            (CacheErrorPolicy::Propagate, Some(error)) => Err(error),
            _ => Ok(()),
        }
    }
}

#[async_trait]
impl CacheDriver for Multi {
    async fn ping(&self) -> CacheResult<()> {
        // Readiness must report an unavailable layer even in Ignore mode.
        Self::check_results(
            "ping",
            join_all(self.layers.iter().map(|layer| layer.ping())).await,
            CacheErrorPolicy::Propagate,
        )
    }

    async fn contains_key(&self, key: &str) -> CacheResult<bool> {
        for (index, layer) in self.layers.iter().enumerate() {
            match layer.contains_key(key).await {
                Ok(true) => return Ok(true),
                Ok(false) => {}
                Err(error) => self.read_error("contains_key", index, error)?,
            }
        }
        Ok(false)
    }

    async fn get(&self, key: &str) -> CacheResult<Option<String>> {
        for (index, layer) in self.layers.iter().enumerate() {
            match layer.get(key).await {
                Ok(Some(value)) => return Ok(Some(value)),
                Ok(None) => {}
                Err(error) => self.read_error("get", index, error)?,
            }
        }
        Ok(None)
    }

    async fn insert(&self, key: &str, value: &str) -> CacheResult<()> {
        Self::check_results(
            "insert",
            join_all(self.layers.iter().map(|layer| layer.insert(key, value))).await,
            self.error_policy,
        )
    }

    async fn insert_with_expiry(
        &self,
        key: &str,
        value: &str,
        duration: Duration,
    ) -> CacheResult<()> {
        Self::check_results(
            "insert_with_expiry",
            join_all(
                self.layers
                    .iter()
                    .map(|layer| layer.insert_with_expiry(key, value, duration)),
            )
            .await,
            self.error_policy,
        )
    }

    async fn remove(&self, key: &str) -> CacheResult<()> {
        Self::check_results(
            "remove",
            join_all(self.layers.iter().map(|layer| layer.remove(key))).await,
            self.error_policy,
        )
    }

    async fn clear(&self) -> CacheResult<()> {
        Self::check_results(
            "clear",
            join_all(self.layers.iter().map(|layer| layer.clear())).await,
            self.error_policy,
        )
    }
}

#[cfg(all(test, feature = "cache_inmem"))]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    use super::*;
    use crate::cache::drivers::inmem::Inmem;

    #[derive(Clone)]
    struct TestLayer {
        store: Arc<Inmem>,
        calls: Arc<Mutex<Vec<&'static str>>>,
        expiries: Arc<Mutex<Vec<Duration>>>,
        offline: Arc<AtomicBool>,
    }

    impl TestLayer {
        fn new() -> Self {
            Self {
                store: Arc::new(Inmem::new(10)),
                calls: Arc::default(),
                expiries: Arc::default(),
                offline: Arc::default(),
            }
        }

        fn call(&self, operation: &'static str) -> CacheResult<()> {
            self.calls.lock().unwrap().push(operation);
            if self.offline.load(Ordering::Relaxed) {
                Err(CacheError::Any("offline".into()))
            } else {
                Ok(())
            }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl CacheDriver for TestLayer {
        async fn ping(&self) -> CacheResult<()> {
            self.call("ping")
        }

        async fn contains_key(&self, key: &str) -> CacheResult<bool> {
            self.call("contains_key")?;
            self.store.contains_key(key).await
        }

        async fn get(&self, key: &str) -> CacheResult<Option<String>> {
            self.call("get")?;
            self.store.get(key).await
        }

        async fn insert(&self, key: &str, value: &str) -> CacheResult<()> {
            self.call("insert")?;
            self.store.insert(key, value).await
        }

        async fn insert_with_expiry(
            &self,
            key: &str,
            value: &str,
            duration: Duration,
        ) -> CacheResult<()> {
            self.call("insert_with_expiry")?;
            self.expiries.lock().unwrap().push(duration);
            self.store.insert_with_expiry(key, value, duration).await
        }

        async fn remove(&self, key: &str) -> CacheResult<()> {
            self.call("remove")?;
            self.store.remove(key).await
        }

        async fn clear(&self) -> CacheResult<()> {
            self.call("clear")?;
            self.store.clear().await
        }
    }

    fn chain(first: &TestLayer, second: &TestLayer, policy: CacheErrorPolicy) -> Multi {
        Multi::new(
            vec![Box::new(first.clone()), Box::new(second.clone())],
            policy,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn reads_stop_at_the_first_hit() {
        let first = TestLayer::new();
        let second = TestLayer::new();
        first.store.insert("key", "memory").await.unwrap();
        second.offline.store(true, Ordering::Relaxed);
        let multi = chain(&first, &second, CacheErrorPolicy::Propagate);

        assert_eq!(multi.get("key").await.unwrap().as_deref(), Some("memory"));
        assert!(multi.contains_key("key").await.unwrap());
        assert_eq!(first.calls(), ["get", "contains_key"]);
        assert!(second.calls().is_empty());
    }

    #[tokio::test]
    async fn reads_fall_through_on_misses_without_backfilling() {
        let first = TestLayer::new();
        let second = TestLayer::new();
        second.store.insert("key", "redis").await.unwrap();
        let multi = chain(&first, &second, CacheErrorPolicy::Propagate);

        assert_eq!(multi.get("key").await.unwrap().as_deref(), Some("redis"));
        assert!(multi.contains_key("key").await.unwrap());
        assert_eq!(multi.get("missing").await.unwrap(), None);
        assert!(!multi.contains_key("missing").await.unwrap());
        assert_eq!(first.calls(), second.calls());
        assert_eq!(first.store.get("key").await.unwrap(), None);
        // Expiry or invalidation of the lower layer must not leave a stale copy.
        second.store.remove("key").await.unwrap();
        assert_eq!(multi.get("key").await.unwrap(), None);
    }

    #[tokio::test]
    async fn writes_and_invalidations_reach_every_layer() {
        let first = TestLayer::new();
        let second = TestLayer::new();
        let multi = chain(&first, &second, CacheErrorPolicy::Propagate);

        multi.insert("key", "old").await.unwrap();
        multi.insert("key", "new").await.unwrap();
        for layer in [&first, &second] {
            assert_eq!(
                layer.store.get("key").await.unwrap().as_deref(),
                Some("new")
            );
        }
        multi.remove("key").await.unwrap();
        for layer in [&first, &second] {
            assert_eq!(layer.store.get("key").await.unwrap(), None);
        }
        multi.insert("other", "value").await.unwrap();
        multi.clear().await.unwrap();
        multi.ping().await.unwrap();
        for layer in [&first, &second] {
            assert_eq!(layer.store.get("other").await.unwrap(), None);
            assert_eq!(
                layer.calls(),
                ["insert", "insert", "remove", "insert", "clear", "ping"]
            );
        }
    }

    #[tokio::test]
    async fn expiring_writes_preserve_the_duration_on_every_layer() {
        let first = TestLayer::new();
        let second = TestLayer::new();
        let multi = chain(&first, &second, CacheErrorPolicy::Propagate);
        let ttl = Duration::from_millis(100);

        multi.insert_with_expiry("key", "value", ttl).await.unwrap();
        for layer in [&first, &second] {
            assert_eq!(*layer.expiries.lock().unwrap(), [ttl]);
            assert_eq!(
                layer.store.get("key").await.unwrap().as_deref(),
                Some("value")
            );
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(multi.get("key").await.unwrap(), None);
        assert!(!multi.contains_key("key").await.unwrap());
    }

    #[tokio::test]
    async fn read_errors_follow_the_configured_policy() {
        for policy in [CacheErrorPolicy::Propagate, CacheErrorPolicy::Ignore] {
            let first = TestLayer::new();
            let second = TestLayer::new();
            first.offline.store(true, Ordering::Relaxed);
            second.store.insert("key", "value").await.unwrap();
            let multi = chain(&first, &second, policy);

            match policy {
                CacheErrorPolicy::Propagate => {
                    assert!(matches!(
                        multi.get("key").await,
                        Err(CacheError::Layer { index: 0, .. })
                    ));
                    assert!(multi.contains_key("key").await.is_err());
                    assert!(second.calls().is_empty());
                }
                CacheErrorPolicy::Ignore => {
                    assert_eq!(multi.get("key").await.unwrap().as_deref(), Some("value"));
                    assert!(multi.contains_key("key").await.unwrap());
                    second.offline.store(true, Ordering::Relaxed);
                    assert_eq!(multi.get("key").await.unwrap(), None);
                    assert!(!multi.contains_key("key").await.unwrap());
                }
            }
            // Ignore mode must still expose unhealthy dependencies to readiness.
            assert!(multi.ping().await.is_err());
            assert!(second.calls().contains(&"ping"));
        }
    }

    #[tokio::test]
    async fn failed_mutations_still_attempt_all_layers() {
        for policy in [CacheErrorPolicy::Propagate, CacheErrorPolicy::Ignore] {
            for failed_index in 0..2 {
                let layers = [TestLayer::new(), TestLayer::new()];
                layers[failed_index].offline.store(true, Ordering::Relaxed);
                let multi = chain(&layers[0], &layers[1], policy);
                let healthy = &layers[1 - failed_index].store;
                let should_fail = policy == CacheErrorPolicy::Propagate;

                let result = multi.insert("key", "value").await;
                assert_eq!(result.is_err(), should_fail);
                if let Err(CacheError::Layer { index, .. }) = result {
                    assert_eq!(index, failed_index);
                }
                assert_eq!(healthy.get("key").await.unwrap().as_deref(), Some("value"));
                assert_eq!(
                    multi
                        .insert_with_expiry("key", "new", Duration::from_secs(60))
                        .await
                        .is_err(),
                    should_fail
                );
                assert_eq!(healthy.get("key").await.unwrap().as_deref(), Some("new"));
                assert_eq!(multi.remove("key").await.is_err(), should_fail);
                assert_eq!(healthy.get("key").await.unwrap(), None);
                healthy.insert("other", "value").await.unwrap();
                assert_eq!(multi.clear().await.is_err(), should_fail);
                assert_eq!(healthy.get("other").await.unwrap(), None);
                assert_eq!(layers[0].calls(), layers[1].calls());
            }
        }
    }

    #[tokio::test]
    async fn complete_outage_returns_the_first_error_unless_ignored() {
        for policy in [CacheErrorPolicy::Propagate, CacheErrorPolicy::Ignore] {
            let first = TestLayer::new();
            let second = TestLayer::new();
            first.offline.store(true, Ordering::Relaxed);
            second.offline.store(true, Ordering::Relaxed);
            let multi = chain(&first, &second, policy);
            let result = multi.insert("key", "value").await;
            if policy == CacheErrorPolicy::Propagate {
                assert!(matches!(result, Err(CacheError::Layer { index: 0, .. })));
            } else {
                assert!(result.is_ok());
            }
            assert_eq!(first.calls(), ["insert"]);
            assert_eq!(second.calls(), ["insert"]);
        }
    }

    #[cfg(feature = "cache_redis")]
    #[tokio::test]
    async fn ignore_policy_does_not_hide_initialization_errors() {
        let config = MultiCacheConfig {
            layers: vec![CacheConfig::Redis(crate::config::RedisCacheConfig {
                uri: "not-a-redis-url".to_string(),
                max_size: 1,
            })],
            error_policy: CacheErrorPolicy::Ignore,
        };
        assert!(new(&config).await.is_err());
    }

    #[tokio::test]
    async fn configuration_builds_a_cache_usable_through_the_public_api() {
        let mut config = crate::tests_cfg::config::test_config();
        config.cache = serde_yaml::from_str("kind: Multi\nlayers:\n  - kind: InMem\n    max_capacity: 1\n  - kind: InMem\n    max_capacity: 10\n").unwrap();
        let cache = crate::cache::create_cache_provider(&config).await.unwrap();
        cache.insert("key", &vec![1, 2, 3]).await.unwrap();
        cache.insert("evict", &true).await.unwrap();
        assert_eq!(
            cache.get::<Vec<i32>>("key").await.unwrap(),
            Some(vec![1, 2, 3])
        );
        let value: Vec<i32> = cache
            .get_or_insert("key", async { panic!("must use the fallback hit") })
            .await
            .unwrap();
        assert_eq!(value, vec![1, 2, 3]);
        assert_eq!(
            cache
                .get_or_insert_with_expiry("new", Duration::from_secs(60), async { Ok(42) })
                .await
                .unwrap(),
            42
        );
        assert_eq!(cache.get::<i32>("new").await.unwrap(), Some(42));
    }

    #[tokio::test]
    async fn a_single_layer_is_supported_but_an_empty_chain_is_not() {
        assert!(Multi::new(vec![], CacheErrorPolicy::Propagate).is_err());
        let config = MultiCacheConfig {
            layers: vec![CacheConfig::InMem(crate::config::InMemCacheConfig {
                max_capacity: 10,
            })],
            error_policy: CacheErrorPolicy::Propagate,
        };
        let cache = new(&config).await.unwrap();
        cache.insert("key", &42).await.unwrap();
        assert_eq!(cache.get::<i32>("key").await.unwrap(), Some(42));
    }

    #[cfg(feature = "cache_redis")]
    #[tokio::test]
    async fn memory_and_redis_chain() {
        let (uri, _container) = crate::tests_cfg::redis::setup_redis_container().await;
        let redis_config = crate::config::RedisCacheConfig { uri, max_size: 2 };
        let redis = super::super::redis::new(&redis_config).await.unwrap();
        let mut config = crate::tests_cfg::config::test_config();
        config.cache = CacheConfig::Multi(MultiCacheConfig {
            layers: vec![
                CacheConfig::InMem(crate::config::InMemCacheConfig { max_capacity: 1 }),
                CacheConfig::Redis(redis_config),
            ],
            error_policy: CacheErrorPolicy::Propagate,
        });
        let cache = crate::cache::create_cache_provider(&config).await.unwrap();
        cache.ping().await.unwrap();

        cache.insert("key", &"value").await.unwrap();
        assert_eq!(
            redis.get::<String>("key").await.unwrap().as_deref(),
            Some("value")
        );
        redis.remove("key").await.unwrap();
        assert_eq!(
            cache.get::<String>("key").await.unwrap().as_deref(),
            Some("value")
        );

        // A different write evicts the memory entry, forcing the Redis fallback.
        redis.insert("key", &"redis value").await.unwrap();
        cache.insert("evict", &true).await.unwrap();
        assert!(cache.contains_key("key").await.unwrap());
        assert_eq!(
            cache.get::<String>("key").await.unwrap().as_deref(),
            Some("redis value")
        );
        redis.remove("key").await.unwrap();
        assert_eq!(cache.get::<String>("key").await.unwrap(), None);

        cache
            .insert_with_expiry("expiring", &42, Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(redis.get::<i32>("expiring").await.unwrap(), Some(42));
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert_eq!(cache.get::<i32>("expiring").await.unwrap(), None);

        cache.insert("remove", &true).await.unwrap();
        cache.remove("remove").await.unwrap();
        assert!(!cache.contains_key("remove").await.unwrap());
        assert!(!redis.contains_key("remove").await.unwrap());
        cache.insert("clear", &true).await.unwrap();
        cache.clear().await.unwrap();
        assert!(!cache.contains_key("clear").await.unwrap());
        assert!(!redis.contains_key("clear").await.unwrap());
        assert!(!redis.contains_key("evict").await.unwrap());
    }
}
