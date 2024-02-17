use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap, StatusCode},
    RequestPartsExt,
};
use axum_extra::extract::{cookie::Key, PrivateCookieJar};
use loco_rs::{
    app::AppContext,
    config::AuthorizationCodeConfig,
    oauth2_store::{basic::BasicTokenResponse, TokenResponse},
    prelude::*,
    Error,
};
use serde::{Deserialize, Serialize};

use crate::models::{sessions, users};

const COOKIE_NAME: &str = "sid";

// Define a struct to represent user from session information serialized
// to/from JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct OAuth2CookieUser {
    pub user: users::Model,
}

// Implement the FromRequestParts trait for the OAuthCookieUser struct
#[async_trait]
impl<S> FromRequestParts<S> for OAuth2CookieUser
where
    S: Send + Sync,
    AppContext: FromRef<S>,
{
    type Rejection = (StatusCode, String);
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> core::result::Result<Self, Self::Rejection> {
        let state: AppContext = AppContext::from_ref(state);
        let jar = PrivateCookieJar::from_headers(&parts.headers, state.key.clone());

        let cookie = jar
            .get(COOKIE_NAME)
            .map(|cookie| cookie.value().to_owned())
            .ok_or_else(|| {
                tracing::info!("Cannot get cookie");
                (StatusCode::UNAUTHORIZED, "Unauthorized!".to_string())
            })?;
        // Check if the session id is expired or exists
        let expired = sessions::Model::is_expired(&state.db, &cookie)
            .await
            .map_err(|e| {
                tracing::info!("Cannot find cookie");
                (StatusCode::UNAUTHORIZED, e.to_string())
            })?;
        if expired {
            tracing::info!("Session expired");
            return Err((StatusCode::UNAUTHORIZED, "Unauthorized!".to_string()));
        }
        let user = users::Model::find_by_session_id(&state.db, &cookie)
            .await
            .map_err(|e| {
                tracing::info!("Cannot find user");
                (StatusCode::UNAUTHORIZED, e.to_string())
            })?;
        Ok(Self { user })
    }
}

/// Set the token with a short live cookie
///
/// # Arguments
/// config - The authorization code config with the oauth2 authorization code
/// grant configuration token - The token response from the oauth2 authorization
/// code grant jar - The private cookie jar
///
/// # Returns
/// A result with the private cookie jar
///
/// # Errors
/// When url parsing fails
pub fn set_token_with_short_live_cookie(
    config: &AuthorizationCodeConfig,
    token: BasicTokenResponse,
    jar: PrivateCookieJar,
) -> Result<PrivateCookieJar> {
    // Set the cookie
    let secs: i64 = token
        .expires_in()
        .unwrap_or(std::time::Duration::new(0, 0))
        .as_secs()
        .try_into()
        .map_err(|_e| Error::InternalServerError)?;
    // domain
    let protected_url = url::Url::parse("http://localhost:3000/oauth2/protected")
        .map_err(|_e| Error::InternalServerError)?;
    let domain = protected_url.domain().unwrap_or("localhost");
    let path = protected_url.path();
    // Create the cookie with the session id, domain, path, and secure flag from
    // the token and profile
    let cookie = axum_extra::extract::cookie::Cookie::build((
        COOKIE_NAME,
        token.access_token().secret().to_owned(),
    ))
    .domain(domain.to_owned())
    .path(path.to_owned())
    // secure flag is for https - https://datatracker.ietf.org/doc/html/rfc6749#section-3.1.2.1
    .secure(true)
    // Restrict access in the client side code to prevent XSS attacks
    .http_only(true)
    .max_age(time::Duration::seconds(secs));
    Ok(jar.add(cookie))
}
