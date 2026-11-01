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
    /// Null cache
    #[default]
    Null,
}

/// Configuration for an ordered chain of cache drivers.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultiCacheConfig {
    /// Non-empty list of cache layers in lookup order. Each layer must be
    /// `InMem` or `Redis`; nested `Multi` and `Null` layers are rejected.
    pub layers: Vec<CacheConfig>,
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
