//! Axum middleware for validating token header
//!
//! # Example:
//!
//! ```
//! use loco_rs::prelude::*;
//! use serde::Serialize;
//! use axum::extract::State;
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

use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap},
};
use axum_extra::extract::cookie;
use serde::{Deserialize, Serialize};

use crate::{
    app::AppContext, auth, config::JWT as JWTConfig, errors::Error, model::Authenticable,
    Result as LocoResult,
};

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
#[derive(Debug, Deserialize, Serialize)]
pub struct JWTWithUser<T: Authenticable> {
    pub claims: auth::jwt::UserClaims,
    pub user: T,
}

// Implement the FromRequestParts trait for the Auth struct
#[async_trait]
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
                    .map_err(|_| Error::Unauthorized("token is not valid".to_string()))?;
                Ok(Self {
                    claims: claims.claims,
                    user,
                })
            }
            Err(_err) => {
                return Err(Error::Unauthorized("token is not valid".to_string()));
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
#[async_trait]
impl<S> FromRequestParts<S> for JWT
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        let ctx: AppContext = AppContext::from_ref(state); // change to ctx

        let token = extract_token(get_jwt_from_config(&ctx)?, parts)?;

        let jwt_secret = ctx.config.get_jwt_config()?;

        match auth::jwt::JWT::new(&jwt_secret.secret).validate(&token) {
            Ok(claims) => Ok(Self {
                claims: claims.claims,
            }),
            Err(_err) => {
                return Err(Error::Unauthorized("token is not valid".to_string()));
            }
        }
    }
}

/// extract JWT token from context configuration
///
/// # Errors
/// Return an error when JWT token not configured
fn get_jwt_from_config(ctx: &AppContext) -> LocoResult<&JWTConfig> {
    ctx.config
        .auth
        .as_ref()
        .ok_or_else(|| Error::string("auth not configured"))?
        .jwt
        .as_ref()
        .ok_or_else(|| Error::string("JWT token not configured"))
}
/// extract token from the configured jwt location settings
fn extract_token(jwt_config: &JWTConfig, parts: &Parts) -> LocoResult<String> {
    #[allow(clippy::match_wildcard_for_single_variants)]
    match jwt_config
        .location
        .as_ref()
        .unwrap_or(&crate::config::JWTLocation::Bearer)
    {
        crate::config::JWTLocation::Cookie { name } => extract_token_from_cookie(name, parts),
        crate::config::JWTLocation::Bearer => extract_token_from_header(&parts.headers)
            .map_err(|e| Error::Unauthorized(e.to_string())),
    }
}
/// Function to extract a token from the authorization header
///
/// # Errors
///
/// When token is not valid or out found
pub fn extract_token_from_header(headers: &HeaderMap) -> LocoResult<String> {
    Ok(headers
        .get(AUTH_HEADER)
        .ok_or_else(|| Error::Unauthorized(format!("header {AUTH_HEADER} token not found")))?
        .to_str()
        .map_err(|err| Error::Unauthorized(err.to_string()))?
        .strip_prefix(TOKEN_PREFIX)
        .ok_or_else(|| Error::Unauthorized(format!("error strip {AUTH_HEADER} value")))?
        .to_string())
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

// ---------------------------------------
//
// API Token Auth / Extractor
//
// ---------------------------------------
#[derive(Debug, Deserialize, Serialize)]
// Represents the data structure for the API token.
pub struct ApiToken<T: Authenticable> {
    pub user: T,
}

#[async_trait]
// Implementing the `FromRequestParts` trait for `ApiToken` to enable extracting
// it from the request.
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
            .map_err(|e| Error::Unauthorized(e.to_string()))?;

        Ok(Self { user })
    }
}

#[cfg(test)]
mod tests {

    use insta::assert_debug_snapshot;
    use rstest::rstest;

    use super::*;
    use crate::config;

    #[rstest]
    #[case("extract_from_default", "https://loco.rs", None)]
    #[case("extract_from_bearer", "loco.rs", Some(config::JWTLocation::Bearer))]
    #[case("extract_from_cookie", "https://loco.rs", Some(config::JWTLocation::Cookie{name: "loco_cookie_key".to_string()}))]
    fn can_extract_token(
        #[case] test_name: &str,
        #[case] url: &str,
        #[case] location: Option<config::JWTLocation>,
    ) {
        let jwt_config = JWTConfig {
            location,
            secret: String::new(),
            expiration: 1,
        };

        let request = axum::http::Request::builder()
            .uri(url)
            .header(AUTH_HEADER, format!("{TOKEN_PREFIX} bearer_token_value"))
            .header(
                "Cookie",
                format!("{}={}", "loco_cookie_key", "cookie_token_value"),
            )
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();
        assert_debug_snapshot!(test_name, extract_token(&jwt_config, &parts));

        // expected error
        let request = axum::http::Request::builder()
            .uri("https://loco.rs")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();
        assert!(extract_token(&jwt_config, &parts).is_err());
    }
}
