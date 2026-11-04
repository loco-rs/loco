use serde::{Deserialize, Serialize};

/// Cache configurations for the application
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum CacheConfig {
    #[cfg(feature = "cache_inmem")]
    /// In-memory cache
    InMem(InMemCacheConfig),
    #[cfg(feature = "cache_redis")]
    /// Redis cache
    Redis(RedisCacheConfig),
    /// Ordered cache layers, checked from first to last on reads.
    Multi(MultiCacheConfig),
    #[cfg(feature = "with-db")]
    /// Cache stored in the application database (Postgres or `SQLite`)
    Database(DatabaseCacheConfig),
    /// Null cache
    #[default]
    Null,
}

/// Configuration for an ordered chain of cache drivers.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultiCacheConfig {
    /// Non-empty list of cache layers in lookup order. Each layer must be
    /// `InMem`, `Redis`, or `Database`; nested `Multi` and `Null` layers are rejected.
    pub layers: Vec<CacheConfig>,
    /// Required positive TTL, in seconds, for `insert` and `get_or_insert` writes.
    /// Explicit expiry methods use their supplied duration instead.
    pub default_ttl_seconds: u64,
    /// How to handle runtime layer failures. Startup and health-check errors
    /// are always returned, regardless of this policy.
    #[serde(default)]
    pub error_policy: CacheErrorPolicy,
}

/// Runtime error handling for a multi-layer cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum CacheErrorPolicy {
    /// Return read errors immediately. Mutations attempt every layer before
    /// returning the first error.
    #[default]
    Propagate,
    /// Log errors and continue. Failed reads count as misses; mutations may
    /// only succeed on some layers, or on none of them.
    Ignore,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InMemCacheConfig {
    #[serde(default = "cache_in_mem_max_capacity")]
    pub max_capacity: u64,
}

fn cache_in_mem_max_capacity() -> u64 {
    32 * 1024 * 1024
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisCacheConfig {
    pub uri: String,
    /// Sets the maximum number of connections managed by the pool.
    pub max_size: u32,
}

/// Settings of the database cache (`cache.kind: Database`). Both fields have
/// defaults, so `kind: Database` alone is a complete configuration.
#[cfg(feature = "with-db")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseCacheConfig {
    /// Sweep expired rows after this many writes. `0` turns the sweep off.
    #[serde(default = "cache_db_sweep_every")]
    pub sweep_every: u64,
    /// The most expired rows one sweep deletes.
    #[serde(default = "cache_db_sweep_batch")]
    pub sweep_batch: u64,
}

#[cfg(feature = "with-db")]
impl Default for DatabaseCacheConfig {
    fn default() -> Self {
        Self {
            sweep_every: cache_db_sweep_every(),
            sweep_batch: cache_db_sweep_batch(),
        }
    }
}

#[cfg(feature = "with-db")]
fn cache_db_sweep_every() -> u64 {
    100
}

#[cfg(feature = "with-db")]
fn cache_db_sweep_batch() -> u64 {
    1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "cache_inmem")]
    #[test]
    fn multi_defaults_to_propagating_errors_and_round_trips() {
        let config: CacheConfig = serde_yaml::from_str(
            "kind: Multi\ndefault_ttl_seconds: 60\nlayers:\n  - kind: InMem\n",
        )
        .unwrap();
        let CacheConfig::Multi(multi) = &config else {
            panic!("expected Multi")
        };
        assert_eq!(multi.error_policy, CacheErrorPolicy::Propagate);
        assert_eq!(multi.default_ttl_seconds, 60);
        assert!(
            matches!(&multi.layers[0], CacheConfig::InMem(layer) if layer.max_capacity == 32 * 1024 * 1024)
        );
        let serialized = serde_yaml::to_string(&config).unwrap();
        let decoded: CacheConfig = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            serde_json::to_value(decoded).unwrap()
        );
    }

    #[test]
    fn multi_requires_layers_ttl_and_a_known_error_policy() {
        assert!(
            serde_yaml::from_str::<CacheConfig>("kind: Multi\ndefault_ttl_seconds: 60").is_err()
        );
        let missing_ttl =
            serde_yaml::from_str::<CacheConfig>("kind: Multi\nlayers: []").unwrap_err();
        assert!(missing_ttl.to_string().contains("default_ttl_seconds"));
        assert!(serde_yaml::from_str::<CacheConfig>(
            "kind: Multi\ndefault_ttl_seconds: 60\nlayers: []\nerror_policy: typo",
        )
        .is_err());
        let CacheConfig::Multi(config) = serde_yaml::from_str::<CacheConfig>(
            "kind: Multi\ndefault_ttl_seconds: 60\nlayers: []\nerror_policy: Ignore",
        )
        .unwrap() else {
            panic!("expected Multi")
        };
        assert_eq!(config.error_policy, CacheErrorPolicy::Ignore);
    }

    #[rstest::rstest]
    #[case("kind: Multi\ndefault_ttl_seconds: 60\nlayers: []")]
    #[case("kind: Multi\ndefault_ttl_seconds: 60\nlayers: [{kind: 'Null'}]")]
    #[case("kind: Multi\ndefault_ttl_seconds: 60\nlayers: [{kind: Multi, default_ttl_seconds: 60, layers: []}]")]
    #[tokio::test]
    async fn multi_rejects_invalid_chains_at_startup(#[case] yaml: &str) {
        let CacheConfig::Multi(config) = serde_yaml::from_str(yaml).unwrap() else {
            panic!("expected Multi")
        };
        assert!(matches!(
            crate::cache::drivers::multi::new(&config).await,
            Err(crate::cache::CacheError::InvalidConfiguration(_))
        ));
    }

    #[cfg(feature = "cache_inmem")]
    #[rstest::rstest]
    #[case("Propagate")]
    #[case("Ignore")]
    #[tokio::test]
    async fn multi_rejects_a_zero_default_ttl_at_startup(#[case] error_policy: &str) {
        let CacheConfig::Multi(config) = serde_yaml::from_str(&format!(
            "kind: Multi\ndefault_ttl_seconds: 0\nlayers: [{{kind: InMem}}]\nerror_policy: {error_policy}",
        ))
        .unwrap() else {
            panic!("expected Multi")
        };
        assert!(matches!(
            crate::cache::drivers::multi::new(&config).await,
            Err(crate::cache::CacheError::InvalidConfiguration(_))
        ));
    }

    #[cfg(feature = "with-db")]
    #[test]
    fn database_cache_needs_nothing_but_its_kind() {
        let CacheConfig::Database(cfg) = serde_yaml::from_str("kind: Database").unwrap() else {
            panic!("expected the Database kind");
        };
        assert_eq!((cfg.sweep_every, cfg.sweep_batch), (100, 1000));

        let CacheConfig::Database(cfg) =
            serde_yaml::from_str("kind: Database\nsweep_every: 5\nsweep_batch: 7").unwrap()
        else {
            panic!("expected the Database kind");
        };
        assert_eq!((cfg.sweep_every, cfg.sweep_batch), (5, 7));
    }
}
