#[cfg(test)]
mod tests {
    use crate::infra_cfg;
    use axum::extract::State;
    use axum::routing::get;
    use loco_rs::app::AppContext;
    use loco_rs::cache::CacheResult;
    use loco_rs::prelude::{get_available_port, get_base_url_port, Response};
    use loco_rs::tests_cfg::db::fail_connection;
    use loco_rs::{cache, controller::health, tests_cfg};
    use serde_json::Value;
    use std::sync::Arc;
    use std::time::Duration;

    pub struct AlwaysFailCache;

    #[async_trait::async_trait]
    impl cache::CacheDriver for AlwaysFailCache {
        async fn ping(&self) -> CacheResult<()> {
            Err(cache::CacheError::Any("Redis connection failed".into()))
        }

        async fn contains_key(&self, _key: &str) -> CacheResult<bool> {
            Err(cache::CacheError::Any("Redis connection failed".into()))
        }

        async fn get(&self, _key: &str) -> CacheResult<Option<String>> {
            Err(cache::CacheError::Any("Redis connection failed".into()))
        }

        async fn insert(&self, _key: &str, _value: &str) -> CacheResult<()> {
            Err(cache::CacheError::Any("Redis connection failed".into()))
        }

        async fn insert_with_expiry(
            &self,
            _key: &str,
            _value: &str,
            _duration: Duration,
        ) -> CacheResult<()> {
            Err(cache::CacheError::Any("Redis connection failed".into()))
        }

        async fn remove(&self, _key: &str) -> CacheResult<()> {
            Err(cache::CacheError::Any("Redis connection failed".into()))
        }

        async fn clear(&self) -> CacheResult<()> {
            Err(cache::CacheError::Any("Redis connection failed".into()))
        }
    }

    #[cfg(not(any(
        feature = "with-db",
        feature = "bg_pg",
        feature = "bg_redis",
        feature = "bg_sqlt",
        feature = "cache_redis",
        feature = "cache_inmem"
    )))]
    #[tokio::test]
    async fn health_no_features() {
        // Compile-time assertions to ensure no features are enabled
        #[cfg(any(
            feature = "with-db",
            feature = "bg_pg",
            feature = "bg_redis",
            feature = "bg_sqlt",
            feature = "cache_redis",
            feature = "cache_inmem"
        ))]
        compile_error!("This test should only run when no features are enabled");

        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.queue_provider = None;

        #[allow(clippy::items_after_statements)]
        async fn action(State(ctx): State<AppContext>) -> loco_rs::Result<Response> {
            health::health(State(ctx)).await
        }

        let port = get_available_port().await;
        let handle =
            infra_cfg::server::start_with_route(ctx, "_health", get(action), Some(port)).await;

        let res = reqwest::get(&format!("{}_health", get_base_url_port(port)))
            .await
            .expect("Valid response");

        assert_eq!(res.status(), 200);

        let res_json: Value = res.json().await.expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);

        handle.abort();
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn health_with_db_success() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.queue_provider = None;

        #[allow(clippy::items_after_statements)]
        async fn action(State(ctx): State<AppContext>) -> loco_rs::Result<Response> {
            health::health(State(ctx)).await
        }

        let port = get_available_port().await;
        let handle =
            infra_cfg::server::start_with_route(ctx, "_health", get(action), Some(port)).await;

        let res = reqwest::get(&format!("{}_health", get_base_url_port(port)))
            .await
            .expect("Valid response");

        assert_eq!(res.status(), 200);

        let res_json: Value = res.json().await.expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);

        handle.abort();
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn health_with_db_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.queue_provider = None;

        ctx.db = fail_connection().await;

        #[allow(clippy::items_after_statements)]
        async fn action(State(ctx): State<AppContext>) -> loco_rs::Result<Response> {
            health::health(State(ctx)).await
        }

        let port = get_available_port().await;
        let handle =
            infra_cfg::server::start_with_route(ctx, "_health", get(action), Some(port)).await;

        let res = reqwest::get(&format!("{}_health", get_base_url_port(port)))
            .await
            .expect("Valid response");

        assert_eq!(res.status(), 200);

        let res_json: Value = res.json().await.expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);

        handle.abort();
    }

    #[cfg(feature = "cache_inmem")]
    #[tokio::test]
    async fn health_with_cache_inmem() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.queue_provider = None;

        let cache = cache::drivers::inmem::new(&loco_rs::config::InMemCacheConfig {
            max_capacity: 32 * 1024 * 1024,
        });
        ctx.cache = cache.into();

        #[allow(clippy::items_after_statements)]
        async fn action(State(ctx): State<AppContext>) -> loco_rs::Result<Response> {
            health::health(State(ctx)).await
        }

        let port = get_available_port().await;
        let handle =
            infra_cfg::server::start_with_route(ctx, "_health", get(action), Some(port)).await;

        let res = reqwest::get(&format!("{}_health", get_base_url_port(port)))
            .await
            .expect("Valid response");

        assert_eq!(res.status(), 200);

        let res_json: Value = res.json().await.expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);

        handle.abort();
    }

    #[cfg(feature = "cache_redis")]
    #[tokio::test]
    async fn health_with_cache_redis_success() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.queue_provider = None;

        #[allow(clippy::items_after_statements)]
        async fn action(State(ctx): State<AppContext>) -> loco_rs::Result<Response> {
            health::health(State(ctx)).await
        }

        let port = get_available_port().await;
        let handle =
            infra_cfg::server::start_with_route(ctx, "_health", get(action), Some(port)).await;

        let res = reqwest::get(&format!("{}_health", get_base_url_port(port)))
            .await
            .expect("Valid response");

        assert_eq!(res.status(), 200);

        let res_json: Value = res.json().await.expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);

        handle.abort();
    }

    #[cfg(feature = "cache_redis")]
    #[tokio::test]
    async fn health_with_cache_redis_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.queue_provider = None;

        ctx.cache = Arc::new(cache::Cache {
            driver: Box::new(AlwaysFailCache),
        });

        #[allow(clippy::items_after_statements)]
        async fn action(State(ctx): State<AppContext>) -> loco_rs::Result<Response> {
            health::health(State(ctx)).await
        }

        let port = get_available_port().await;
        let handle =
            infra_cfg::server::start_with_route(ctx, "_health", get(action), Some(port)).await;

        let res = reqwest::get(&format!("{}_health", get_base_url_port(port)))
            .await
            .expect("Valid response");

        assert_eq!(res.status(), 200);

        let res_json: Value = res.json().await.expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);

        handle.abort();
    }

    #[cfg(all(feature = "with-db", feature = "cache_redis"))]
    #[tokio::test]
    async fn health_with_db_and_redis_success() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.queue_provider = None;

        #[allow(clippy::items_after_statements)]
        async fn action(State(ctx): State<AppContext>) -> loco_rs::Result<Response> {
            health::health(State(ctx)).await
        }

        let port = get_available_port().await;
        let handle =
            infra_cfg::server::start_with_route(ctx, "_health", get(action), Some(port)).await;

        let res = reqwest::get(&format!("{}_health", get_base_url_port(port)))
            .await
            .expect("Valid response");

        assert_eq!(res.status(), 200);

        let res_json: Value = res.json().await.expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);

        handle.abort();
    }

    #[cfg(all(feature = "with-db", feature = "cache_redis"))]
    #[tokio::test]
    async fn health_with_db_and_redis_partial_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.queue_provider = None;

        ctx.cache = Arc::new(cache::Cache {
            driver: Box::new(AlwaysFailCache),
        });

        #[allow(clippy::items_after_statements)]
        async fn action(State(ctx): State<AppContext>) -> loco_rs::Result<Response> {
            health::health(State(ctx)).await
        }

        let port = get_available_port().await;
        let handle =
            infra_cfg::server::start_with_route(ctx, "_health", get(action), Some(port)).await;

        let res = reqwest::get(&format!("{}_health", get_base_url_port(port)))
            .await
            .expect("Valid response");

        assert_eq!(res.status(), 200);

        let res_json: Value = res.json().await.expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);

        handle.abort();
    }
}
