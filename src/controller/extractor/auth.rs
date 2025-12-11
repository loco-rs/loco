//! Axum middleware for validating token header
//!
//! # Example:
//!
//! ```
//! use loco_rs::prelude::*;
//! use serde::Serialize;
//! use axum::extract::State;
//! use loco_rs::controller::extractor::auth;
//!
//! #[derive(Serialize)]
//! pub struct TestResponse {
//!     pub pid: String,
//! }
//!
//! async fn current(
//!     auth: auth::JWT,
//!     State(ctx): State<AppContext>,
//! ) -> Result<Response> {
//!     format::json(TestResponse{ pid: auth.claims.pid})
//! }
//! ```
use std::collections::HashMap;

use axum::{
    extract::{FromRef, FromRequestParts, Query},
    http::{request::Parts, HeaderMap},
};
use axum_extra::extract::cookie;
use serde::{Deserialize, Serialize};
use tracing;

use crate::{app::AppContext, auth, config::JWT as JWTConfig, errors::Error, Result as LocoResult};

#[cfg(feature = "with-db")]
use crate::model::{Authenticable, ModelError};

// ---------------------------------------
//
// JWT Auth extractor
//
// ---------------------------------------

// Define constants for token prefix and authorization header
const TOKEN_PREFIX: &str = "Bearer ";
const AUTH_HEADER: &str = "authorization";

// Define a struct to represent user authentication information serialized
// to/from JSON
#[cfg(feature = "with-db")]
#[derive(Debug, Deserialize, Serialize)]
pub struct JWTWithUser<T: Authenticable> {
    pub claims: auth::jwt::UserClaims,
    pub user: T,
}

// Implement the FromRequestParts trait for the Auth struct
#[cfg(feature = "with-db")]
impl<S, T> FromRequestParts<S> for JWTWithUser<T>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
    T: Authenticable,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        let ctx: AppContext = AppContext::from_ref(state);

        let token = extract_token(get_jwt_from_config(&ctx)?, parts)?;

        let jwt_secret = ctx.config.get_jwt_config()?;

        match auth::jwt::JWT::new(&jwt_secret.secret).validate(&token) {
            Ok(claims) => {
                let user = T::find_by_claims_key(&ctx.db, &claims.claims.pid)
                    .await
                    .map_err(|e| match e {
                        ModelError::EntityNotFound => Error::Unauthorized("not found".to_string()),
                        ModelError::DbErr(db_err) => {
                            tracing::error!("Database error during authentication: {}", db_err);
                            Error::InternalServerError
                        }
                        _ => {
                            tracing::error!("Authentication error: {}", e);
                            Error::Unauthorized("could not authorize".to_string())
                        }
                    })?;
                Ok(Self {
                    claims: claims.claims,
                    user,
                })
            }
            Err(err) => {
                tracing::error!("JWT validation error: {}", err);
                Err(Error::Unauthorized("token is not valid".to_string()))
            }
        }
    }
}

// Define a struct to represent user authentication information serialized
// to/from JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct JWT {
    pub claims: auth::jwt::UserClaims,
}

// Implement the FromRequestParts trait for the Auth struct
impl<S> FromRequestParts<S> for JWT
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        extract_jwt_from_request_parts(parts, state)
    }
}

/// extract a [JWT] token from request parts, using a non-mutable reference to the [Parts]
///
/// # Errors
/// Return an error when JWT token not configured or when the token is not valid
pub fn extract_jwt_from_request_parts<S>(parts: &Parts, state: &S) -> Result<JWT, Error>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    let ctx: AppContext = AppContext::from_ref(state); // change to ctx

    let token = extract_token(get_jwt_from_config(&ctx)?, parts)?;

    let jwt_secret = ctx.config.get_jwt_config()?;

    match auth::jwt::JWT::new(&jwt_secret.secret).validate(&token) {
        Ok(claims) => Ok(JWT {
            claims: claims.claims,
        }),
        Err(err) => {
            tracing::error!("JWT validation error: {}", err);
            Err(Error::Unauthorized("token is not valid".to_string()))
        }
    }
}

/// extract JWT token from context configuration
///
/// # Errors
/// Return an error when JWT token not configured
pub fn get_jwt_from_config(ctx: &AppContext) -> LocoResult<&JWTConfig> {
    ctx.config
        .auth
        .as_ref()
        .ok_or_else(|| Error::string("auth not configured"))?
        .jwt
        .as_ref()
        .ok_or_else(|| Error::string("JWT token not configured"))
}
/// extract token from the configured jwt location settings
///
/// # Errors
///
/// Returns an error when the token cannot be extracted from any of the configured locations,
/// such as missing headers, invalid formats, or inaccessible request data.
pub fn extract_token(jwt_config: &JWTConfig, parts: &Parts) -> LocoResult<String> {
    let locations = get_jwt_locations(jwt_config.location.as_ref());

    for location in &locations {
        if let Ok(token) = extract_token_from_location(location, parts) {
            return Ok(token);
        }
    }

    // If we get here, none of the locations worked
    Err(Error::Unauthorized("Token not found in any of the configured JWT locations. Please check your auth.jwt.location configuration.".to_string()))
}

/// Get the list of JWT locations to try, with Bearer as default
fn get_jwt_locations(
    config: Option<&crate::config::JWTLocationConfig>,
) -> Vec<&crate::config::JWTLocation> {
    match config {
        Some(crate::config::JWTLocationConfig::Single(location)) => vec![location],
        Some(crate::config::JWTLocationConfig::Multiple(locations)) => locations.iter().collect(),
        None => vec![&crate::config::JWTLocation::Bearer],
    }
}

/// Extract token from a specific location
fn extract_token_from_location(
    location: &crate::config::JWTLocation,
    parts: &Parts,
) -> LocoResult<String> {
    match location {
        crate::config::JWTLocation::Query { name } => extract_token_from_query(name, parts),
        crate::config::JWTLocation::Cookie { name } => extract_token_from_cookie(name, parts),
        crate::config::JWTLocation::Bearer => extract_token_from_header(&parts.headers),
    }
}

/// Function to extract a token from the authorization header
///
/// # Errors
///
/// When token is not valid or not found
pub fn extract_token_from_header(headers: &HeaderMap) -> LocoResult<String> {
    let token = headers
        .get(AUTH_HEADER)
        .ok_or_else(|| Error::Unauthorized(format!("header {AUTH_HEADER} token not found")))?
        .to_str()
        .map_err(|err| Error::Unauthorized(err.to_string()))?
        .strip_prefix(TOKEN_PREFIX)
        .ok_or_else(|| Error::Unauthorized(format!("error strip {AUTH_HEADER} value")))?;

    Ok(token.to_string())
}

/// Extract a token value from cookie
///
/// # Errors
/// when token value from cookie is not found
pub fn extract_token_from_cookie(name: &str, parts: &Parts) -> LocoResult<String> {
    // LogoResult
    let jar: cookie::CookieJar = cookie::CookieJar::from_headers(&parts.headers);
    Ok(jar
        .get(name)
        .ok_or(Error::Unauthorized("token is not found".to_string()))?
        .to_string()
        .strip_prefix(&format!("{name}="))
        .ok_or_else(|| Error::Unauthorized("error strip value".to_string()))?
        .to_string())
}
/// Extract a token value from query
///
/// # Errors
/// when token value from cookie is not found
pub fn extract_token_from_query(name: &str, parts: &Parts) -> LocoResult<String> {
    // LogoResult
    let parameters: Query<HashMap<String, String>> =
        Query::try_from_uri(&parts.uri).map_err(|err| Error::Unauthorized(err.to_string()))?;
    parameters
        .get(name)
        .cloned()
        .ok_or_else(|| Error::Unauthorized(format!("`{name}` query parameter not found")))
}

// ---------------------------------------
//
// API Token Auth / Extractor
//
// ---------------------------------------
#[cfg(feature = "with-db")]
#[derive(Debug, Deserialize, Serialize)]
// Represents the data structure for the API token.
pub struct ApiToken<T: Authenticable> {
    pub user: T,
}

// Implementing the `FromRequestParts` trait for `ApiToken` to enable extracting
// it from the request.
#[cfg(feature = "with-db")]
impl<S, T> FromRequestParts<S> for ApiToken<T>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
    T: Authenticable,
{
    type Rejection = Error;

    // Extracts `ApiToken` from the request parts.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        // Extract API key from the request header.
        let api_key = extract_token_from_header(&parts.headers)?;

        // Convert the state reference to the application context.
        let state: AppContext = AppContext::from_ref(state);

        // Retrieve user information based on the API key from the database.
        let user = T::find_by_api_key(&state.db, &api_key)
            .await
            .map_err(|e| match e {
                ModelError::EntityNotFound => Error::Unauthorized("not found".to_string()),
                ModelError::DbErr(db_err) => {
                    tracing::error!("Database error during API key authentication: {}", db_err);
                    Error::InternalServerError
                }
                _ => {
                    tracing::error!("API key authentication error: {}", e);
                    Error::Unauthorized("could not authorize".to_string())
                }
            })?;

        Ok(Self { user })
    }
}

#[cfg(test)]
mod tests {

    use axum::http::{HeaderMap, HeaderValue};

    use super::*;
    use crate::config;

    #[test]
    fn test_extract_token_from_header_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTH_HEADER,
            HeaderValue::from_str("Bearer valid_token_123").unwrap(),
        );

        let result = extract_token_from_header(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid_token_123");
    }

    #[test]
    fn test_extract_token_from_header_with_spaces() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTH_HEADER,
            HeaderValue::from_str("Bearer  token_with_spaces  ").unwrap(),
        );

        let result = extract_token_from_header(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " token_with_spaces  ");
    }

    #[test]
    fn test_extract_token_from_header_special_chars() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTH_HEADER,
            HeaderValue::from_str("Bearer token-with_special.chars").unwrap(),
        );

        let result = extract_token_from_header(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "token-with_special.chars");
    }

    #[test]
    fn test_extract_token_from_header_missing_header() {
        let headers = HeaderMap::new();
        let result = extract_token_from_header(&headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("authorization token not found"));
    }

    #[test]
    fn test_extract_token_from_header_missing_bearer_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTH_HEADER,
            HeaderValue::from_str("InvalidPrefix token").unwrap(),
        );

        let result = extract_token_from_header(&headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("error strip authorization value"));
    }

    #[test]
    fn test_extract_token_from_header_empty_value() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTH_HEADER, HeaderValue::from_str("Bearer").unwrap());

        let result = extract_token_from_header(&headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("error strip authorization value"));
    }

    #[test]
    fn test_extract_token_from_cookie_success() {
        let request = axum::http::Request::builder()
            .header("Cookie", "test_cookie=cookie_value_123")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_cookie("test_cookie", &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "cookie_value_123");
    }

    #[test]
    fn test_extract_token_from_cookie_special_chars() {
        let request = axum::http::Request::builder()
            .header("Cookie", "auth_token=token-with.special_chars")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_cookie("auth_token", &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "token-with.special_chars");
    }

    #[test]
    fn test_extract_token_from_cookie_missing_cookie() {
        let request = axum::http::Request::builder().body(()).unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_cookie("nonexistent", &parts);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("token is not found"));
    }

    #[test]
    fn test_extract_token_from_cookie_not_found() {
        let request = axum::http::Request::builder()
            .header("Cookie", "different_cookie=value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_cookie("nonexistent", &parts);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("token is not found"));
    }

    #[test]
    fn test_extract_token_from_query_success() {
        let request = axum::http::Request::builder()
            .uri("https://example.com?token=query_value_123&other=param")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_query("token", &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "query_value_123");
    }

    #[test]
    fn test_extract_token_from_query_special_chars() {
        let request = axum::http::Request::builder()
            .uri("https://example.com?auth_token=token-with.special_chars")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_query("auth_token", &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "token-with.special_chars");
    }

    #[test]
    fn test_extract_token_from_query_missing_param() {
        let request = axum::http::Request::builder()
            .uri("https://example.com?other=param")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_query("nonexistent_param", &parts);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("query parameter not found"));
    }

    #[test]
    fn test_extract_token_from_query_invalid_uri() {
        let request = axum::http::Request::builder()
            .uri("not-a-valid-uri")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_query("nonexistent_param", &parts);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_jwt_locations_default() {
        let jwt_config = JWTConfig {
            location: None,
            secret: String::new(),
            expiration: 1,
        };

        let locations = get_jwt_locations(jwt_config.location.as_ref());
        assert_eq!(locations.len(), 1);
        assert!(matches!(locations[0], config::JWTLocation::Bearer));
    }

    #[test]
    fn test_get_jwt_locations_single_bearer() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Single(
                config::JWTLocation::Bearer,
            )),
            secret: String::new(),
            expiration: 1,
        };

        let locations = get_jwt_locations(jwt_config.location.as_ref());
        assert_eq!(locations.len(), 1);
        assert!(matches!(locations[0], config::JWTLocation::Bearer));
    }

    #[test]
    fn test_get_jwt_locations_single_cookie() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Single(
                config::JWTLocation::Cookie {
                    name: "auth_token".to_string(),
                },
            )),
            secret: String::new(),
            expiration: 1,
        };

        let locations = get_jwt_locations(jwt_config.location.as_ref());
        assert_eq!(locations.len(), 1);
        assert!(matches!(locations[0], config::JWTLocation::Cookie { .. }));
    }

    #[test]
    fn test_get_jwt_locations_single_query() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Single(
                config::JWTLocation::Query {
                    name: "token".to_string(),
                },
            )),
            secret: String::new(),
            expiration: 1,
        };

        let locations = get_jwt_locations(jwt_config.location.as_ref());
        assert_eq!(locations.len(), 1);
        assert!(matches!(locations[0], config::JWTLocation::Query { .. }));
    }

    #[test]
    fn test_get_jwt_locations_multiple() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Multiple(vec![
                config::JWTLocation::Cookie {
                    name: "auth".to_string(),
                },
                config::JWTLocation::Query {
                    name: "token".to_string(),
                },
                config::JWTLocation::Bearer,
            ])),
            secret: String::new(),
            expiration: 1,
        };

        let locations = get_jwt_locations(jwt_config.location.as_ref());
        assert_eq!(locations.len(), 3);
        assert!(matches!(locations[0], config::JWTLocation::Cookie { .. }));
        assert!(matches!(locations[1], config::JWTLocation::Query { .. }));
        assert!(matches!(locations[2], config::JWTLocation::Bearer));
    }

    #[test]
    fn test_extract_token_from_location_bearer() {
        let request = axum::http::Request::builder()
            .uri("https://example.com?token=query_value")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_value"))
            .header("Cookie", "auth_token=cookie_value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_location(&config::JWTLocation::Bearer, &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " bearer_value");
    }

    #[test]
    fn test_extract_token_from_location_cookie() {
        let request = axum::http::Request::builder()
            .uri("https://example.com?token=query_value")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_value"))
            .header("Cookie", "auth_token=cookie_value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_location(
            &config::JWTLocation::Cookie {
                name: "auth_token".to_string(),
            },
            &parts,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "cookie_value");
    }

    #[test]
    fn test_extract_token_from_location_query() {
        let request = axum::http::Request::builder()
            .uri("https://example.com?token=query_value")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_value"))
            .header("Cookie", "auth_token=cookie_value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token_from_location(
            &config::JWTLocation::Query {
                name: "token".to_string(),
            },
            &parts,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "query_value");
    }

    #[test]
    fn test_extract_token_single_location_success() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Single(
                config::JWTLocation::Bearer,
            )),
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("https://example.com")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} valid_token"))
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " valid_token");
    }

    #[test]
    fn test_extract_token_multiple_locations_fallback() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Multiple(vec![
                config::JWTLocation::Cookie {
                    name: "nonexistent".to_string(),
                },
                config::JWTLocation::Query {
                    name: "token".to_string(),
                },
            ])),
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("https://example.com?token=fallback_token")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fallback_token");
    }

    #[test]
    fn test_extract_token_all_locations_fail() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Multiple(vec![
                config::JWTLocation::Cookie {
                    name: "nonexistent".to_string(),
                },
                config::JWTLocation::Query {
                    name: "missing".to_string(),
                },
            ])),
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("https://example.com")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Token not found in any of the configured JWT locations"));
    }

    #[test]
    fn test_extract_from_default() {
        let jwt_config = JWTConfig {
            location: None,
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("https://loco.rs")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_token_value"))
            .header("Cookie", "loco_cookie_key=cookie_token_value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " bearer_token_value");
    }

    #[test]
    fn test_extract_from_bearer() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Single(
                config::JWTLocation::Bearer,
            )),
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("loco.rs")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_token_value"))
            .header("Cookie", "loco_cookie_key=cookie_token_value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " bearer_token_value");
    }

    #[test]
    fn test_extract_from_cookie() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Single(
                config::JWTLocation::Cookie {
                    name: "loco_cookie_key".to_string(),
                },
            )),
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("https://loco.rs")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_token_value"))
            .header("Cookie", "loco_cookie_key=cookie_token_value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "cookie_token_value");
    }

    #[test]
    fn test_extract_from_query() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Single(
                config::JWTLocation::Query {
                    name: "query_token".to_string(),
                },
            )),
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("https://loco.rs?query_token=query_token_value&test=loco")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_token_value"))
            .header("Cookie", "loco_cookie_key=cookie_token_value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "query_token_value");
    }

    #[test]
    fn test_extract_from_multiple_locations() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Multiple(vec![
                config::JWTLocation::Cookie {
                    name: "nonexistent".to_string(),
                },
                config::JWTLocation::Query {
                    name: "query_token".to_string(),
                },
            ])),
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("https://loco.rs?query_token=query_token_value&test=loco")
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_token_value"))
            .header("Cookie", "loco_cookie_key=cookie_token_value")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "query_token_value");
    }

    #[test]
    fn test_extract_token_error_message_for_missing_token() {
        let jwt_config = JWTConfig {
            location: Some(config::JWTLocationConfig::Multiple(vec![
                config::JWTLocation::Cookie {
                    name: "nonexistent".to_string(),
                },
                config::JWTLocation::Query {
                    name: "missing".to_string(),
                },
            ])),
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri("https://loco.rs")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();

        let result = extract_token(&jwt_config, &parts);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("auth.jwt.location configuration"));
    }
}
