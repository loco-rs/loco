//! This module contains a base routes related to health checks and status
//! reporting. These routes are commonly used to monitor the health of the
//! application and its dependencies.

use axum::{extract::State, response::Response, routing::get};
use serde::Serialize;

use super::{format, routes::Routes};
use crate::{app::AppContext, db, redis, Result};

/// Represents the health status of the application.
#[derive(Serialize)]
struct Health {
    pub ok: bool,
    pub redis: bool,
    pub db: bool,
}

/// Check the healthiness of the application bt ping to the redis and the DB to
/// insure that connection
async fn health(State(ctx): State<AppContext>) -> Result<Response> {
    let db_ok = match ctx.db.ping().await {
        Ok(()) => {
            tracing::info!("health_db_ping_success");
            true
        }
        Err(error) => {
            tracing::error!(err.msg = %error, err.detail = ?error, "health_db_ping_error");
            false
        }
    };
    let mut redis_ok = true;
    if let Some(pool) = ctx.redis {
        if let Err(error) = redis::ping(&pool).await {
            tracing::error!(err.msg = %error, err.detail = ?error, "health_redis_ping_error");
            redis_ok = false;
        } else {
            tracing::info!("health_redis_ping_success");
        }
    }
    format::json(Health {
        ok: db_ok && redis_ok,
        db: db_ok,
        redis: redis_ok,
    })
}

/// Defines and returns the health-related routes.
pub fn routes() -> Routes {
    Routes::new().add("/_health", get(health))
}
