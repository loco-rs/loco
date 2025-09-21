//! This module contains a base routes related to readiness checks and status
//! reporting. These routes are commonly used to monitor the readiness of the
//! application and its dependencies.

use super::{format, routes::Routes};
#[cfg(any(feature = "cache_inmem", feature = "cache_redis"))]
use crate::config;
use crate::{app::AppContext, Result};
use axum::{extract::State, response::Response, routing::get};
use serde::Serialize;

/// Represents the health status of the application.
#[derive(Serialize)]
pub struct Health {
    pub ok: bool,
}

/// Check application ping endpoint
///
/// # Errors
/// This function always returns `Ok` with a JSON response indicating the
pub async fn ping() -> Result<Response> {
    format::json(Health { ok: true })
}

/// Check application ping endpoint
///
/// # Errors
/// This function always returns `Ok` with a JSON response indicating the
pub async fn health() -> Result<Response> {
    format::json(Health { ok: true })
}

/// Check the readiness of the application by sending a ping request to
/// Redis or the DB (depending on feature flags) to ensure connection liveness.
///
/// # Errors
/// All errors are logged, and the readiness status is returned as a JSON response.
pub async fn readiness(State(ctx): State<AppContext>) -> Result<Response> {
    // Check database connection
    #[cfg(feature = "with-db")]
    if let Err(error) = &ctx.db.ping().await {
        tracing::error!(err.msg = %error, err.detail = ?error, "readiness_db_ping_error");
        return format::json(Health { ok: false });
    }

    // Check queue connection
    if let Some(queue) = &ctx.queue_provider {
        if let Err(error) = queue.ping().await {
            tracing::error!(err.msg = %error, err.detail = ?error, "readiness_queue_ping_error");
            return format::json(Health { ok: false });
        }
    }

    // Check cache connection
    #[cfg(any(feature = "cache_inmem", feature = "cache_redis"))]
    {
        match ctx.config.cache {
            #[cfg(feature = "cache_inmem")]
            config::CacheConfig::InMem(_) => {
                if let Err(error) = &ctx.cache.driver.ping().await {
                    tracing::error!(err.msg = %error, err.detail = ?error, "readiness_cache_ping_error");
                    return format::json(Health { ok: false });
                }
            }
            #[cfg(feature = "cache_redis")]
            config::CacheConfig::Redis(_) => {
                if let Err(error) = &ctx.cache.driver.ping().await {
                    tracing::error!(err.msg = %error, err.detail = ?error, "readiness_cache_ping_error");
                    return format::json(Health { ok: false });
                }
            }
            config::CacheConfig::Null => (),
        }
    }

    format::json(Health { ok: true })
}

/// Defines and returns the readiness-related routes.
pub fn routes() -> Routes {
    Routes::new()
        .add("/_readiness", get(readiness))
        .add("/_ping", get(ping))
        .add("/_health", get(health))
}

#[cfg(test)]
mod tests {
    use axum::routing::get;
    use loco_rs::tests_cfg::db::fail_connection;
    use loco_rs::{bgworker, cache, config, controller::monitoring, tests_cfg};
    use serde_json::Value;
    use tower::ServiceExt;

    #[cfg(feature = "cache_redis")]
    use crate::tests_cfg::redis::setup_redis_container;

    #[tokio::test]
    async fn ping_works() {
        let ctx = tests_cfg::app::get_app_context().await;

        // Create a router with the ping route
        let router = axum::Router::new()
            .route("/_ping", get(monitoring::ping))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_ping")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[tokio::test]
    async fn health_works() {
        let ctx = tests_cfg::app::get_app_context().await;

        // Create a router with the health route
        let router = axum::Router::new()
            .route("/_health", get(monitoring::health))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_health")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(not(feature = "with-db"))]
    #[tokio::test]
    async fn readiness_no_features() {
        let ctx = tests_cfg::app::get_app_context().await;

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn readiness_with_db_success() {
        let ctx = tests_cfg::app::get_app_context().await;

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn readiness_with_db_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        ctx.db = fail_connection().await;

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);
    }

    #[cfg(feature = "cache_inmem")]
    #[tokio::test]
    async fn readiness_with_cache_inmem() {
        let mut ctx = tests_cfg::app::get_app_context().await;

        ctx.cache = cache::drivers::inmem::new(&loco_rs::config::InMemCacheConfig {
            max_capacity: 32 * 1024 * 1024,
        })
        .into();

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "cache_redis")]
    #[tokio::test]
    async fn readiness_with_cache_redis_success() {
        let (redis_url, _container) = setup_redis_container().await;
        let mut ctx = tests_cfg::app::get_app_context().await;

        // Create Redis cache driver and assign to ctx.cache
        let redis_cache = cache::drivers::redis::new(&config::RedisCacheConfig {
            uri: redis_url,
            max_size: 10,
        })
        .await
        .expect("Failed to create Redis cache");
        ctx.cache = redis_cache.into();

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "cache_redis")]
    #[tokio::test]
    async fn readiness_with_cache_redis_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        let failour_redis_url = "redis://127.0.0.2:0";
        // Force config to Redis to ensure ping path executes, but swap driver to Null (which errors on ping)
        ctx.config.cache = config::CacheConfig::Redis(loco_rs::config::RedisCacheConfig {
            uri: failour_redis_url.to_string(),
            max_size: 10,
        });
        // Create Redis cache driver and assign to ctx.cache
        ctx.cache = cache::drivers::redis::new(&config::RedisCacheConfig {
            uri: failour_redis_url.to_string(),
            max_size: 10,
        })
        .await
        .expect("Failed to create Redis cache")
        .into();

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);
    }

    #[tokio::test]
    async fn readiness_with_queue_not_present() {
        let mut ctx = tests_cfg::app::get_app_context().await;
        // simulate background queue mode with a no-op provider
        ctx.config.workers.mode = config::WorkerMode::BackgroundQueue;
        ctx.queue_provider = Some(std::sync::Arc::new(bgworker::Queue::None));

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], true);
    }

    #[cfg(feature = "bg_redis")]
    #[tokio::test]
    async fn readiness_with_queue_present_failure() {
        let mut ctx = tests_cfg::app::get_app_context().await;

        // Configure Redis queue with invalid URL to trigger failure
        let failure_redis_url = "redis://127.0.0.2:0";
        ctx.config.workers.mode = config::WorkerMode::BackgroundQueue;
        ctx.config.queue = Some(config::QueueConfig::Redis(config::RedisQueueConfig {
            uri: failure_redis_url.to_string(),
            dangerously_flush: false,
            queues: None,
            num_workers: 1,
        }));

        // Create Redis queue provider directly with failing Redis connection
        ctx.queue_provider = Some(std::sync::Arc::new(
            bgworker::redis::create_provider(&config::RedisQueueConfig {
                uri: failure_redis_url.to_string(),
                dangerously_flush: false,
                queues: None,
                num_workers: 1,
            })
            .await
            .expect("Failed to create Redis queue provider"),
        ));

        // Create a router with the readiness route
        let router = axum::Router::new()
            .route("/_readiness", get(monitoring::readiness))
            .with_state(ctx);

        // Create a request
        let req = axum::http::Request::builder()
            .uri("/_readiness")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();

        // Test the router directly using oneshot
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), 200);

        // Get the response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res_json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(res_json["ok"], false);
    }
}
