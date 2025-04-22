use crate::{
    app::AppContext,
    cache,
    environment::Environment,
    storage::{self, Storage},
    tests_cfg::config::test_config,
};

pub async fn get_app_context() -> AppContext {
    // Always use null cache for tests to avoid feature-specific complications
    let driver = cache::drivers::null::new();
    let cache = cache::Cache::new(driver);

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
