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
    #[cfg(feature = "with-db")]
    /// Cache stored in the application database (Postgres or `SQLite`)
    Database(DatabaseCacheConfig),
    /// Null cache
    #[default]
    Null,
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

#[cfg(all(test, feature = "with-db"))]
mod tests {
    use super::*;

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
