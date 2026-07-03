use serde::{Deserialize, Serialize};

/// Cache configurations for the application
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum CacheConfig {
    #[cfg(feature = "cache_inmem")]
    /// In-memory cache
    InMem(InMemCacheConfig),
    #[cfg(feature = "cache_redis")]
    /// Redis cache
    Redis(RedisCacheConfig),
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
