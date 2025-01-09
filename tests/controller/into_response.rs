use loco_rs::{controller, prelude::*, tests_cfg};
use serial_test::serial;

use crate::infra_cfg;

#[tokio::test]
#[serial]
async fn not_found() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        controller::not_found()
    }

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;

    let res = reqwest::get(infra_cfg::server::get_base_url())
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 404);

    let res_text = res.text().await.expect("response text");
    let res_json: serde_json::Value = serde_json::from_str(&res_text).expect("Valid JSON response");

    let expected_json = serde_json::json!({
        "error": "not_found",
        "description": "Resource was not found"
    });

    assert_eq!(res_json, expected_json);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn internal_server_error() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        Err(Error::InternalServerError)
    }

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;

    let res = reqwest::get(infra_cfg::server::get_base_url())
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 500);

    let res_text = res.text().await.expect("response text");
    let res_json: serde_json::Value = serde_json::from_str(&res_text).expect("Valid JSON response");

    let expected_json = serde_json::json!({
        "error": "internal_server_error",
        "description": "Internal Server Error",
    });

    assert_eq!(res_json, expected_json);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn unauthorized() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        controller::unauthorized("user not unauthorized")
    }

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;

    let res = reqwest::get(infra_cfg::server::get_base_url())
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    let res_text = res.text().await.expect("response text");
    let res_json: serde_json::Value = serde_json::from_str(&res_text).expect("Valid JSON response");

    let expected_json = serde_json::json!({
        "error": "unauthorized",
        "description": "You do not have permission to access this resource"
    });

    assert_eq!(res_json, expected_json);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn fallback() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        Err(Error::Message(String::new()))
    }

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;

    let res = reqwest::get(infra_cfg::server::get_base_url())
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 500);

    let res_text = res.text().await.expect("response text");
    let res_json: serde_json::Value = serde_json::from_str(&res_text).expect("Valid JSON response");

    let expected_json = serde_json::json!({
        "error": "internal_server_error",
        "description": "Internal Server Error",
    });

    assert_eq!(res_json, expected_json);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn custom_error() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        Err(Error::CustomError(
            axum::http::StatusCode::PAYLOAD_TOO_LARGE,
            controller::ErrorDetail {
                error: Some("Payload Too Large".to_string()),
                description: Some("413 Payload Too Large".to_string()),
            },
        ))
    }

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;

    let res = reqwest::get(infra_cfg::server::get_base_url())
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 413);

    let res_text = res.text().await.expect("response text");
    let res_json: serde_json::Value = serde_json::from_str(&res_text).expect("Valid JSON response");

    let expected_json = serde_json::json!({
        "error": "Payload Too Large",
        "description": "413 Payload Too Large"
    });

    assert_eq!(res_json, expected_json);

    handle.abort();
}
