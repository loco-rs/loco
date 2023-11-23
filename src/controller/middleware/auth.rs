//! Axum middleware for validate token header
//!
//! # Example:
//!
//! ```
//! use loco_rs::{
//!     controller::{middleware, format},
//!     app::AppContext,
//!     Result,
//! };
//! use axum::{
//!     Json,
//!     extract::State
//! };
//!
//! pub struct TestResponse {
//!     pub pid: String,
//! }
//! async fn current(
//!     auth: middleware::auth::Auth,
//!     State(ctx): State<AppContext>,
//! ) -> Result<Json<TestResponse>> {
//!     format::json(TestResponse{ pid: auth.claims.pid})
//! }
//! ```

use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap},
};
use serde::{Deserialize, Serialize};

use crate::{app::AppContext, auth, errors::Error};

// Define constants for token prefix and authorization header
const TOKEN_PREFIX: &str = "Bearer ";
const AUTH_HEADER: &str = "authorization";

// Define a struct to represent user authentication information serialized
// to/from JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct Auth {
    pub claims: auth::UserClaims,
}

// Implement the FromRequestParts trait for the Auth struct
#[async_trait]
impl<S> FromRequestParts<S> for Auth
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        let token = extract_token_from_header(&parts.headers)
            .map_err(|e| Error::Unauthorized(e.to_string()))?;

        let state: AppContext = AppContext::from_ref(state);

        match auth::JWT::new(&state.config.auth.secret).validate(&token) {
            Ok(claims) => Ok(Self {
                claims: claims.claims,
            }),
            Err(_err) => {
                return Err(Error::Unauthorized(format!("[Auth] token is not valid.")));
            }
        }
    }
}

// Function to extract a token from the authorization header
fn extract_token_from_header(headers: &HeaderMap) -> eyre::Result<String> {
    Ok(headers
        .get(AUTH_HEADER)
        .ok_or_else(|| eyre::eyre!("header {} token not found", AUTH_HEADER))?
        .to_str()?
        .strip_prefix(TOKEN_PREFIX)
        .ok_or_else(|| eyre::eyre!("error strip {} value", AUTH_HEADER))?
        .to_string())
}
