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

// Mock user struct for testing JWTWithUser extractor
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

// Test handler for JWTWithUser extractor
async fn jwt_with_user_handler(auth: auth::JWTWithUser<TestUser>) -> Result<Response> {
    format::json(TestUserResponse {
        pid: auth.claims.pid,
        user_id: auth.user.id,
        user_email: auth.user.email,
    })
}

// Test JWTWithUser extractor with valid token
#[tokio::test]
async fn can_extract_jwt_with_user_valid_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT token with known PID
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", get(jwt_with_user_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: TestUserResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");
    assert_eq!(body.user_id, 1);
    assert_eq!(body.user_email, "test@example.com");

    handle.abort();
}

// Test JWTWithUser extractor with invalid token
#[tokio::test]
async fn can_handle_jwt_with_user_invalid_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", get(jwt_with_user_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", "Bearer invalid_token")
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    handle.abort();
}

// Test JWTWithUser extractor with non-existent user
#[tokio::test]
async fn can_handle_jwt_with_user_nonexistent_user() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT token with unknown PID
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "unknown_pid".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", get(jwt_with_user_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    handle.abort();
}

// Test JWTWithUser extractor with missing token
#[tokio::test]
async fn can_handle_jwt_with_user_missing_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", get(jwt_with_user_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    handle.abort();
}
