//! Axum middleware for validate token header
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

use crate::{app::AppContext, auth, errors::Error, model::Authenticable};

// ---------------------------------------
//
// JWT Auth extractor from bearer token
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
        let token = extract_token_from_header(&parts.headers)
            .map_err(|e| Error::Unauthorized(e.to_string()))?;

        let state: AppContext = AppContext::from_ref(state);

        let jwt_secret = state.config.get_jwt_config()?;

        match auth::jwt::JWT::new(&jwt_secret.secret).validate(&token) {
            Ok(claims) => {
                let user = T::find_by_claims_key(&state.db, &claims.claims.pid)
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
        let token = extract_token_from_header(&parts.headers)
            .map_err(|e| Error::Unauthorized(e.to_string()))?;

        let state: AppContext = AppContext::from_ref(state);

        let jwt_secret = state.config.get_jwt_config()?;

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

/// Function to extract a token from the authorization header
///
/// # Errors
///
/// When token is not valid or out found
pub fn extract_token_from_header(headers: &HeaderMap) -> eyre::Result<String> {
    Ok(headers
        .get(AUTH_HEADER)
        .ok_or_else(|| eyre::eyre!("header {} token not found", AUTH_HEADER))?
        .to_str()?
        .strip_prefix(TOKEN_PREFIX)
        .ok_or_else(|| eyre::eyre!("error strip {} value", AUTH_HEADER))?
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
        let api_key = extract_token_from_header(&parts.headers)
            .map_err(|e| Error::Unauthorized(e.to_string()))?;

        // Convert the state reference to the application context.
        let state: AppContext = AppContext::from_ref(state);

        // Retrieve user information based on the API key from the database.
        let user = T::find_by_api_key(&state.db, &api_key)
            .await
            .map_err(|e| Error::Unauthorized(e.to_string()))?;

        Ok(Self { user })
    }
}

// ---------------------------------------
//
// JWT Auth extractor from cookie
//
// ---------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct JWTCookie {
    pub claims: auth::jwt::UserClaims,
}

#[async_trait]
impl<S> FromRequestParts<S> for JWTCookie
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        let token = extract_token_from_cookie(parts)?;
        let state: AppContext = AppContext::from_ref(state);

        let jwt_secret = state.config.get_jwt_config()?;

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

#[derive(Debug, Deserialize, Serialize)]
pub struct JWTCookieWithUser<T: Authenticable> {
    pub claims: auth::jwt::UserClaims,
    pub user: T,
}

#[async_trait]
impl<S, T> FromRequestParts<S> for JWTCookieWithUser<T>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
    T: Authenticable,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        let token = extract_token_from_cookie(parts)?;
        let state: AppContext = AppContext::from_ref(state);

        let jwt_secret = state.config.get_jwt_config()?;

        match auth::jwt::JWT::new(&jwt_secret.secret).validate(&token) {
            Ok(claims) => {
                let user = T::find_by_claims_key(&state.db, &claims.claims.pid)
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

/// Extract a token value from cookie
///
/// # Errors
/// when token value from cookie is not found
pub fn extract_token_from_cookie(parts: &Parts) -> eyre::Result<String> {
    let jar = cookie::CookieJar::from_headers(&parts.headers);
    Ok(jar
        .get("token")
        .ok_or(Error::Unauthorized("token is not found".to_string()))?
        .to_string()
        .strip_prefix("token=")
        .ok_or_else(|| eyre::eyre!("error strip {} value", "token"))?
        .to_string())
}
