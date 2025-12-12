use loco_rs::{controller::extractor::auth, prelude::*, tests_cfg};
use serde::{Deserialize, Serialize};

use loco_rs::model::{Authenticable, ModelError};

use crate::infra_cfg;

#[derive(Debug, Deserialize, Serialize)]
pub struct TestUserResponse {
    pub pid: String,
    pub user_id: i32,
    pub user_email: String,
}

// Mock user struct for testing ApiToken extractor
#[derive(Debug, Clone)]
struct TestUser {
    id: i32,
    email: String,
}

#[async_trait::async_trait]
impl Authenticable for TestUser {
    async fn find_by_claims_key(
        _db: &sea_orm::DatabaseConnection,
        pid: &str,
    ) -> Result<Self, ModelError> {
        // Simple mock: return user if pid matches, otherwise not found
        if pid == "test_pid_123" {
            Ok(Self {
                id: 1,
                email: "test@example.com".to_string(),
            })
        } else {
            Err(ModelError::EntityNotFound)
        }
    }

    async fn find_by_api_key(
        _db: &sea_orm::DatabaseConnection,
        api_key: &str,
    ) -> Result<Self, ModelError> {
        // Simple mock: return user if api_key matches, otherwise not found
        if api_key == "test_api_key_123" {
            Ok(Self {
                id: 1,
                email: "test@example.com".to_string(),
            })
        } else {
            Err(ModelError::EntityNotFound)
        }
    }
}

// Test handler for ApiToken extractor
async fn api_token_handler(auth: auth::ApiToken<TestUser>) -> Result<Response> {
    format::json(TestUserResponse {
        pid: String::new(), // API tokens don't have PIDs
        user_id: auth.user.id,
        user_email: auth.user.email,
    })
}

// Test ApiToken extractor with valid API key
#[tokio::test]
async fn can_extract_api_token_valid() {
    let ctx = tests_cfg::app::get_app_context().await;

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", get(api_token_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", "Bearer test_api_key_123")
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: TestUserResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, ""); // API tokens don't have PIDs
    assert_eq!(body.user_id, 1);
    assert_eq!(body.user_email, "test@example.com");

    handle.abort();
}

// Test ApiToken extractor with invalid API key
#[tokio::test]
async fn can_handle_api_token_invalid() {
    let ctx = tests_cfg::app::get_app_context().await;

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", get(api_token_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", "Bearer invalid_api_key")
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    handle.abort();
}

// Test ApiToken extractor with missing Authorization header
#[tokio::test]
async fn can_handle_api_token_missing() {
    let ctx = tests_cfg::app::get_app_context().await;

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", get(api_token_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    handle.abort();
}

// Test response serialization
#[tokio::test]
async fn test_user_response_serialization() {
    let response = TestUserResponse {
        pid: "test_pid".to_string(),
        user_id: 1,
        user_email: "test@example.com".to_string(),
    };

    let json = serde_json::to_string(&response).expect("Should serialize");
    let deserialized: TestUserResponse = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(response.pid, deserialized.pid);
    assert_eq!(response.user_id, deserialized.user_id);
    assert_eq!(response.user_email, deserialized.user_email);
}
