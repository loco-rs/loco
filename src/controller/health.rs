//! This module contains a base routes related to health checks and status
//! reporting. These routes are commonly used to monitor the health of the
//! application and its dependencies.

use aide::axum::routing::get;
use axum::{extract::State, Json};
use schemars::JsonSchema;
use serde::Serialize;

use super::routes::Routes;
use crate::app::AppContext;

/// Represents the health status of the application.
#[derive(Serialize, JsonSchema)]
struct Health {
    pub ok: bool,
}

/// Check the healthiness of the application bt ping to the redis and the DB to
/// insure that connection
async fn health(State(ctx): State<AppContext>) -> Json<Health> {
    let mut is_ok = match ctx.db.ping().await {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(err.msg = %error, err.detail = ?error, "health_db_ping_error");
            false
        }
    };

    if let Some(queue) = ctx.queue_provider {
        if let Err(error) = queue.ping().await {
            tracing::error!(err.msg = %error, err.detail = ?error, "health_redis_ping_error");
            is_ok = false;
        }
    }
    Json(Health { ok: is_ok })
}

/// Defines and returns the health-related routes.
pub fn routes() -> Routes {
    Routes::new().add("/_health", get(health))
}
