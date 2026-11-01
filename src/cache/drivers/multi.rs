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
