//! This module contains a base routes related to health checks and status
//! reporting. These routes are commonly used to monitor the health of the
//! application and its dependencies.

use axum::{extract::State, response::Response, routing::get};
use serde::Serialize;

use super::{format, routes::Routes};
use crate::{app::AppContext, Result};

/// Represents the health status of the application.
#[derive(Serialize)]
struct Health {
    pub ok: bool,
}

/// Check the healthiness of the application by sending a ping request to
/// Redis or the DB (depending on feature flags) to ensure connection liveness.
pub async fn health(State(ctx): State<AppContext>) -> Result<Response> {
    let mut is_ok: bool = true;

    #[cfg(feature = "with-db")]
    if let Err(error) = &ctx.db.ping().await {
        tracing::error!(err.msg = %error, err.detail = ?error, "health_db_ping_error");
        is_ok = false;
    }

    if let Some(queue) = &ctx.queue_provider {
        if let Err(error) = queue.ping().await {
            tracing::error!(err.msg = %error, err.detail = ?error, "health_queue_ping_error");
            is_ok = false;
        }
    }

    #[cfg(any(feature = "cache_inmem", feature = "cache_redis"))]
    if let Err(error) = &ctx.cache.driver.ping().await {
        tracing::error!(err.msg = %error, err.detail = ?error, "health_cache_ping_error");
        is_ok = false;
    }

    format::json(Health { ok: is_ok })
}

/// Defines and returns the health-related routes.
pub fn routes() -> Routes {
    Routes::new().add("/_health", get(health))
}
