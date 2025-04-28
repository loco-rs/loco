use crate::{
    app::AppContext,
    cache,
    environment::Environment,
    storage::{self, Storage},
    tests_cfg::config::test_config,
};
use std::sync::Arc;

pub async fn get_app_context() -> AppContext {
    // Always use in-memory cache for tests if feature is available, otherwise fall back to null
    #[cfg(feature = "cache_inmem")]
    let cache = cache::drivers::inmem::new(&crate::config::InMemCacheConfig {
        max_capacity: 32 * 1024 * 1024, // Use explicit value instead of default
    });

    // If cache_inmem is not enabled, use null cache regardless of other features
    #[cfg(not(feature = "cache_inmem"))]
    let cache = cache::Cache::new(cache::drivers::null::new());

    AppContext {
        environment: Environment::Test,
        #[cfg(feature = "with-db")]
        db: super::db::dummy_connection().await,
        queue_provider: None,
        config: test_config(),
        mailer: None,
        storage: Storage::single(storage::drivers::mem::new()).into(),
        cache: cache.into(),
        container: Arc::new(Default::default()),
    }
}
