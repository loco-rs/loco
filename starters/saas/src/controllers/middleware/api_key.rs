//! Axum middleware for validating API key tokens.

use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use loco_rs::{app::AppContext, controller::middleware::auth, errors::Error};
use serde::{Deserialize, Serialize};

use crate::models::_entities::users;

#[derive(Debug, Deserialize, Serialize)]
// Represents the data structure for the API token.
pub struct ApiToken {
    pub user: users::Model,
}

#[async_trait]
// Implementing the `FromRequestParts` trait for `ApiToken` to enable extracting it from the request.
impl<S> FromRequestParts<S> for ApiToken
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    // Extracts `ApiToken` from the request parts.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        // Extract API key from the request header.
        let api_key = auth::extract_token_from_header(&parts.headers)
            .map_err(|e| Error::Unauthorized(e.to_string()))?;

        // Convert the state reference to the application context.
        let state: AppContext = AppContext::from_ref(state);

        // Retrieve user information based on the API key from the database.
        let user = users::Model::find_by_api_key(&state.db, &api_key).await?;

        Ok(Self { user })
    }
}
