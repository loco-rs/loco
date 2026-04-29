use axum::extract::FromRef;
use loco_rs::{
    app::{AppContext, SharedStore},
    cache,
    prelude::*,
    tests_cfg,
};
use std::sync::Arc;

use crate::infra_cfg;

#[cfg(feature = "with-db")]
use sea_orm::DatabaseConnection;

/// Tests that DatabaseConnection can be extracted from AppContext via FromRef
#[cfg(feature = "with-db")]
#[tokio::test]
async fn can_extract_db_connection_from_app_context() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action(State(ctx): State<AppContext>) -> Result<Response> {
        // Use FromRef to extract DatabaseConnection from AppContext
        let _db: DatabaseConnection = DatabaseConnection::from_ref(&ctx);
        format::json(serde_json::json!({"extracted": "db"}))
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.expect("JSON response");
    assert_eq!(body["extracted"], "db");

    handle.abort();
}

/// Tests that Arc<Cache> can be extracted from AppContext via FromRef
#[tokio::test]
async fn can_extract_cache_from_app_context() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action(State(ctx): State<AppContext>) -> Result<Response> {
        // Use FromRef to extract Arc<Cache> from AppContext
        let _cache: Arc<cache::Cache> = Arc::from_ref(&ctx);
        format::json(serde_json::json!({"extracted": "cache"}))
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.expect("JSON response");
    assert_eq!(body["extracted"], "cache");

    handle.abort();
}

/// Tests that Arc<SharedStore> can be extracted from AppContext via FromRef
#[tokio::test]
async fn can_extract_shared_store_from_app_context() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action(State(ctx): State<AppContext>) -> Result<Response> {
        // Use FromRef to extract Arc<SharedStore> from AppContext
        let _store: Arc<SharedStore> = Arc::from_ref(&ctx);
        format::json(serde_json::json!({"extracted": "shared_store"}))
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.expect("JSON response");
    assert_eq!(body["extracted"], "shared_store");

    handle.abort();
}
