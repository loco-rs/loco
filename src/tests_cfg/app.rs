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
        queue: None,
        config: test_config(),
        mailer: None,
        storage: Storage::single(storage::drivers::mem::new()).into(),
        cache: cache::Cache::new(cache::drivers::inmem::new()).into(),
    }
}
