use crate::{
    app::AppContext,
    cache, config,
    environment::Environment,
    storage::{self, Storage},
    tests_cfg::config::test_config,
};

pub async fn get_app_context() -> AppContext {
    let cache_config = config::InMemCacheConfig {
        max_capacity: 32 * 1024 * 1024,
    };
    let cache = cache::drivers::inmem::new(&cache_config);

    AppContext {
        environment: Environment::Test,
        #[cfg(feature = "with-db")]
        db: super::db::dummy_connection().await,
        queue_provider: None,
        config: test_config(),
        mailer: None,
        storage: Storage::single(storage::drivers::mem::new()).into(),
        cache: cache.into(),
    }
}
