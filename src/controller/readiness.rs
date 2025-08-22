//! This module contains a base routes related to readiness checks and status
//! reporting. These routes are commonly used to monitor the readiness of the
//! application and its dependencies.

use axum::{extract::State, response::Response, routing::get};

use super::{format, routes::Routes};
use crate::controller::response::Health;
use crate::{app::AppContext, Result};

/// Check the readiness of the application by sending a ping request to
/// Redis or the DB (depending on feature flags) to ensure connection liveness.
///
/// # Errors
/// All errors are logged, and the readiness status is returned as a JSON response.
pub async fn readiness(State(ctx): State<AppContext>) -> Result<Response> {
    let mut is_ok: bool = true;

    #[cfg(feature = "with-db")]
    if let Err(error) = &ctx.db.ping().await {
        tracing::error!(err.msg = %error, err.detail = ?error, "readiness_db_ping_error");
        is_ok = false;
    }

    if let Some(queue) = &ctx.queue_provider {
        if let Err(error) = queue.ping().await {
            tracing::error!(err.msg = %error, err.detail = ?error, "readiness_queue_ping_error");
            is_ok = false;
        }
    }

    #[cfg(any(feature = "cache_inmem", feature = "cache_redis"))]
    if let Err(error) = &ctx.cache.driver.ping().await {
        tracing::error!(err.msg = %error, err.detail = ?error, "readiness_cache_ping_error");
        is_ok = false;
    }

    format::json(Health { ok: is_ok })
}

/// Defines and returns the readiness-related routes.
pub fn routes() -> Routes {
    Routes::new().add("/_readiness", get(readiness))
}
