#![allow(clippy::unused_async)]

use axum::{debug_handler, Extension};
use axum_session::{Session, SessionNullPool};
use loco_rs::errors;
use loco_rs::prelude::*;
use loco_rs::request_context::RequestContext;

const REQUEST_CONTEXT_DATA_KEY: &str = "alan";

/// Get a session
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn get_session(_session: Session<SessionNullPool>) -> Result<Response> {
    format::empty()
}

/// Set a request context
///
/// # Errors
///
/// This function will return an error if result fails
///
#[debug_handler]
pub async fn create_request_context(mut req: RequestContext) -> Result<Response> {
    let mut driver = req.driver();
    let data = "turing".to_string();
    driver
        .insert(REQUEST_CONTEXT_DATA_KEY, data.clone())
        .await
        .map_err(|_| errors::Error::InternalServerError)?;
    tracing::info!(
        "Request Context data set - Key: {:?}, Value: {:?}",
        REQUEST_CONTEXT_DATA_KEY,
        data
    );
    Ok(data.into_response())
}

/// Get a request context
///
/// # Errors
///
/// This function will return an error if result fails
///
#[debug_handler]
pub async fn get_request_context(mut req: Extension<RequestContext>) -> Result<Response> {
    let driver = req.driver();
    let data = driver
        .get::<String>(REQUEST_CONTEXT_DATA_KEY)
        .await
        .map_err(|e| errors::Error::InternalServerError)?
        .unwrap_or_default();
    tracing::info!(
        "Request Context data retrieved - Key: {:?}, Value: {:?}",
        REQUEST_CONTEXT_DATA_KEY,
        data
    );
    Ok(data.into_response())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("mysession")
        .add("/", get(get_session))
        .add("/request_context", post(create_request_context))
        .add("/request_context", get(get_request_context))
}
