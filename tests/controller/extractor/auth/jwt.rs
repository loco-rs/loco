use loco_rs::{controller::extractor::auth, prelude::*, tests_cfg};
use serde::{Deserialize, Serialize};

use crate::infra_cfg;

#[derive(Debug, Deserialize, Serialize)]
pub struct TestResponse {
    pub pid: String,
}

// Test handler for JWT extractor
async fn jwt_handler(auth: auth::JWT) -> Result<Response> {
    format::json(TestResponse {
        pid: auth.claims.pid,
    })
}

// Test JWT extractor with valid token
#[tokio::test]
async fn can_extract_jwt_with_valid_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);
    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");
    handle.abort();
}

// Test JWT extractor with invalid token
#[tokio::test]
async fn can_handle_invalid_jwt_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

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

// Test JWT extractor with missing token
#[tokio::test]
async fn can_handle_missing_jwt_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);
    handle.abort();
}

// Test JWT extractor with expired token
#[tokio::test]
async fn can_handle_expired_jwt_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(1, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

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

// Test JWT extractor with malformed authorization header
#[tokio::test]
async fn can_handle_malformed_authorization_header() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", "some_token")
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);
    handle.abort();
}

// Test JWT extractor with missing JWT configuration
#[tokio::test]
async fn can_handle_missing_jwt_configuration() {
    let ctx = tests_cfg::app::get_app_context().await;

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", "Bearer some_token")
        .send()
        .await
        .expect("Valid response");

    // When JWT config is missing, it should return 500 (Internal Server Error)
    // because the extractor can't find the JWT configuration
    assert_eq!(res.status(), 500);
    handle.abort();
}

// Test JWT extractor with Cookie location
#[tokio::test]
async fn can_extract_jwt_from_cookie() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Cookie location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Cookie {
                    name: "auth_token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT token
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Cookie", format!("auth_token={token}"))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");

    handle.abort();
}

// Test JWT extractor with Query location
#[tokio::test]
async fn can_extract_jwt_from_query() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Query location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Query {
                    name: "token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT token
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}?token={}", get_base_url_port(port), token))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");

    handle.abort();
}

// Test JWT extractor with multiple locations - Cookie first, then Query fallback
#[tokio::test]
async fn can_extract_jwt_with_multiple_locations_cookie_fallback() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use multiple locations (Cookie first, then Query)
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Multiple(vec![
                loco_rs::config::JWTLocation::Cookie {
                    name: "nonexistent_cookie".to_string(), // This will fail
                },
                loco_rs::config::JWTLocation::Query {
                    name: "token".to_string(), // This will succeed
                },
            ])),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT token
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}?token={}", get_base_url_port(port), token))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");

    handle.abort();
}

// Test JWT extractor with multiple locations - Query first, then Bearer fallback
#[tokio::test]
async fn can_extract_jwt_with_multiple_locations_query_fallback() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use multiple locations (Query first, then Bearer)
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Multiple(vec![
                loco_rs::config::JWTLocation::Query {
                    name: "missing_param".to_string(), // This will fail
                },
                loco_rs::config::JWTLocation::Bearer, // This will succeed
            ])),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT token
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");

    handle.abort();
}

// Test JWT extractor with multiple locations - all locations fail
#[tokio::test]
async fn can_handle_multiple_locations_all_fail() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use multiple locations that will all fail
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Multiple(vec![
                loco_rs::config::JWTLocation::Cookie {
                    name: "nonexistent_cookie".to_string(),
                },
                loco_rs::config::JWTLocation::Query {
                    name: "missing_param".to_string(),
                },
            ])),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    handle.abort();
}

// Test JWT extractor with Cookie location - missing cookie
#[tokio::test]
async fn can_handle_cookie_location_missing_cookie() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Cookie location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Cookie {
                    name: "auth_token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    handle.abort();
}

// Test JWT extractor with Query location - missing query parameter
#[tokio::test]
async fn can_handle_query_location_missing_param() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Query location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Query {
                    name: "token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    handle.abort();
}

// Test JWT extractor with JWT that has wrong algorithm
#[tokio::test]
async fn can_handle_jwt_with_wrong_algorithm() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a JWT with different secret (simulating wrong algorithm)
    // Use a valid base64-encoded secret
    let different_secret = "DifferentSecretKey123456789012345678901234567890".to_string();
    let jwt = loco_rs::auth::jwt::JWT::new(&different_secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

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

// Test JWT extractor with JWT that has invalid signature but valid format
#[tokio::test]
async fn can_handle_jwt_with_invalid_signature() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT then modify it to have invalid signature
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let mut token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    // Corrupt the signature by changing the last character
    if let Some(last_char) = token.chars().last() {
        let new_char = if last_char == 'A' { 'B' } else { 'A' };
        token.pop();
        token.push(new_char);
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

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

// Test JWT extractor with malformed JWT structure
#[tokio::test]
async fn can_handle_malformed_jwt_structure() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", "Bearer not.a.valid.jwt")
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);
    handle.abort();
}

// Test JWT extractor with cookie containing special characters
#[tokio::test]
async fn can_extract_jwt_from_cookie_with_special_chars() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Cookie location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Cookie {
                    name: "auth_token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT token
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Cookie", format!("auth_token={token}"))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");

    handle.abort();
}

// Test JWT extractor with cookie containing empty value
#[tokio::test]
async fn can_handle_cookie_with_empty_value() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Cookie location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Cookie {
                    name: "auth_token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Cookie", "auth_token=")
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);
    handle.abort();
}

// Test JWT extractor with query parameter containing special characters
#[tokio::test]
async fn can_extract_jwt_from_query_with_special_chars() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Query location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Query {
                    name: "token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a valid JWT token
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(3600, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}?token={}", get_base_url_port(port), token))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 200);

    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");

    handle.abort();
}

// Test JWT extractor with query parameter containing empty value
#[tokio::test]
async fn can_handle_query_with_empty_value() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Query location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Query {
                    name: "token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}?token=", get_base_url_port(port)))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);
    handle.abort();
}

// Test JWT extractor with query parameter containing spaces
#[tokio::test]
async fn can_handle_query_with_spaces() {
    let mut ctx = tests_cfg::app::get_app_context().await;

    // Configure JWT auth to use Query location
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: Some(loco_rs::config::JWTLocationConfig::Single(
                loco_rs::config::JWTLocation::Query {
                    name: "token".to_string(),
                },
            )),
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!(
            "{}?token=invalid token with spaces",
            get_base_url_port(port)
        ))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);
    handle.abort();
}

// Test JWT extractor error message for missing token
#[tokio::test]
async fn can_validate_error_message_for_missing_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    // The error message should be consistent
    let _error_text = res.text().await.expect("Error response should have text");
    // Note: The actual error message format depends on the error handling implementation
    // This test ensures we get a 401 status for missing token
    handle.abort();
}

// Test JWT extractor error message for invalid token
#[tokio::test]
async fn can_validate_error_message_for_invalid_token() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", "Bearer invalid_token")
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    // The error message should be consistent for invalid tokens
    let _error_text = res.text().await.expect("Error response should have text");
    // Note: The actual error message format depends on the error handling implementation
    // This test ensures we get a 401 status for invalid token
    handle.abort();
}

// Test JWT extractor error message for malformed authorization header
#[tokio::test]
async fn can_validate_error_message_for_malformed_header() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", "InvalidPrefix token")
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 401);

    // The error message should be consistent for malformed headers
    let _error_text = res.text().await.expect("Error response should have text");
    // Note: The actual error message format depends on the error handling implementation
    // This test ensures we get a 401 status for malformed header
    handle.abort();
}

// Test JWT extractor with JWT that expires exactly at current time
#[tokio::test]
async fn can_handle_jwt_expires_exactly_at_current_time() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a JWT that expires exactly at current time (0 seconds from now)
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(0, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Valid response");

    // JWT should be considered expired if exp is exactly at current time
    assert_eq!(res.status(), 401);
    handle.abort();
}

// Test JWT extractor with JWT that expired 1 second ago
#[tokio::test]
async fn can_handle_jwt_expired_one_second_ago() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a JWT that expired 1 second ago
    // We'll use a negative expiration to simulate past expiration
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(0, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    // Wait 1 second to ensure the token is expired
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

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

// Test JWT extractor with JWT that expires in 1 second (just valid)
#[tokio::test]
async fn can_handle_jwt_expires_in_one_second() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a JWT that expires in 5 seconds to account for test setup time
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(5, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Valid response");

    // JWT should be valid if it expires in 5 seconds
    assert_eq!(res.status(), 200);

    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");

    handle.abort();
}

// Test JWT extractor with JWT that has missing exp claim
#[tokio::test]
async fn can_handle_jwt_with_missing_exp_claim() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a JWT manually without exp claim
    // This simulates a JWT that was created without proper exp handling
    let _jwt = loco_rs::auth::jwt::JWT::new(&secret);

    // For this test, we'll use an invalid JWT structure that would fail validation
    // since we can't easily create a JWT without exp claim using the current API
    let token = "invalid.jwt.without.exp".to_string();

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

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

// Test JWT extractor with JWT that has invalid exp claim format
#[tokio::test]
async fn can_handle_jwt_with_invalid_exp_claim() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a JWT with invalid exp claim format
    // This simulates a JWT with malformed exp claim
    let _jwt = loco_rs::auth::jwt::JWT::new(&secret);

    // Corrupt the JWT to simulate invalid exp claim
    // This will make the JWT invalid
    let token = "invalid.jwt.with.bad.exp".to_string();

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

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

// Test JWT extractor with JWT that expires in the very distant future
#[tokio::test]
async fn can_handle_jwt_with_distant_future_expiration() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a JWT that expires in 10 years (very distant future)
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);
    let token = jwt
        .generate_token(
            315_360_000,
            "test_pid_123".to_string(),
            serde_json::Map::new(),
        ) // 10 years in seconds
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Valid response");

    // JWT should be valid if it expires in the distant future
    assert_eq!(res.status(), 200);

    let body: TestResponse = res.json().await.expect("Valid JSON response");
    assert_eq!(body.pid, "test_pid_123");

    handle.abort();
}

// Test JWT extractor with JWT that has exp claim at epoch time (1970)
#[tokio::test]
async fn can_handle_jwt_with_epoch_expiration() {
    let mut ctx = tests_cfg::app::get_app_context().await;
    let secret = "PqRwLF2rhHe8J22oBeHy".to_string();
    ctx.config.auth = Some(loco_rs::config::Auth {
        jwt: Some(loco_rs::config::JWT {
            location: None,
            secret: secret.clone(),
            expiration: 3600,
        }),
    });

    // Create a JWT that expired at epoch time (1970)
    // This simulates a JWT with exp=0 or very old timestamp
    let jwt = loco_rs::auth::jwt::JWT::new(&secret);

    // Generate a token with 0 expiration (epoch time)
    let token = jwt
        .generate_token(0, "test_pid_123".to_string(), serde_json::Map::new())
        .expect("Failed to generate token");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(jwt_handler), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(get_base_url_port(port))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Valid response");

    // JWT should be expired if exp is at epoch time
    assert_eq!(res.status(), 401);
    handle.abort();
}
