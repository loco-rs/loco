use crate::{
    app::AppContext,
    cache,
    environment::Environment,
    storage::{self, Storage},
    tests_cfg::config::test_config,
};

pub async fn get_app_context() -> AppContext {
    AppContext {
        environment: Environment::Test,
        #[cfg(feature = "with-db")]
        db: super::db::dummy_connection().await,
        queue_provider: None,
        config: test_config(),
        mailer: None,
        storage: Storage::single(storage::drivers::mem::new()).into(),
        #[cfg(feature = "cache_inmem")]
        cache: cache::Cache::new(cache::drivers::inmem::new()).into(),
        #[cfg(not(feature = "cache_inmem"))]
        cache: cache::Cache::new(cache::drivers::null::new()).into(),
        session_store: None,
    }
}
