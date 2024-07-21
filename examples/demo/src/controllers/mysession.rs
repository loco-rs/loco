#![allow(clippy::unused_async)]

use axum::debug_handler;
use axum_session::{Session, SessionNullPool};
use loco_rs::prelude::*;
use loco_rs::request_context::RequestContext;

/// Get a session
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn get_session(_session: Session<SessionNullPool>) -> Result<Response> {
    format::empty()
}

/// Get a request context
#[debug_handler]
pub async fn get_request_context(mut req: RequestContext) -> Result<Response> {
    let mut driver = req.driver();
    tracing::info!("Request Context: {:?}", driver.get::<String>("alan").await);
    driver.insert("alan", "turing").await.unwrap();
    tracing::info!("Request Context: {:?}", driver.get::<String>("alan").await);
    format::empty()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("mysession")
        .add("/", get(get_session))
        .add("/request_context", get(get_request_context))
}
