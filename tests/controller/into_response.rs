use loco_rs::{controller, prelude::*, tests_cfg};
use serde::{Deserialize, Serialize};

use crate::infra_cfg;

#[tokio::test]
async fn not_found() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        controller::not_found()
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
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
async fn internal_server_error() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        Err(Error::InternalServerError)
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
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
async fn unauthorized() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        controller::unauthorized("user not unauthorized")
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
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
async fn fallback() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        Err(Error::Message(String::new()))
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
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
async fn custom_error() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        Err(Error::CustomError(
            axum::http::StatusCode::PAYLOAD_TOO_LARGE,
            controller::ErrorDetail {
                error: Some("Payload Too Large".to_string()),
                description: Some("413 Payload Too Large".to_string()),
                errors: None,
            },
        ))
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
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

#[tokio::test]
async fn json_rejection() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Data {
        pub email: String,
    }

    #[allow(clippy::items_after_statements)]
    async fn action(Json(_params): Json<Data>) -> Result<Response> {
        format::json(())
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", post(action), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .post(get_base_url_port(port))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 422);

    let res_text = res.text().await.expect("response text");
    let res_json: serde_json::Value = serde_json::from_str(&res_text).expect("Valid JSON response");

    let expected_json = serde_json::json!({
        "error": "Bad Request",
    });

    assert_eq!(res_json, expected_json);

    handle.abort();
}
