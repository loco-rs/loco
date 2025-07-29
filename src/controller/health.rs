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
    #[cfg(feature = "with-db")]
    pub db: bool,
    #[cfg(any(feature = "bg_pg", feature = "bg_redis", feature = "bg_sqlt"))]
    pub queue: bool,
    #[cfg(feature = "cache_redis")]
    pub redis_cache: bool,
}

/// Check the healthiness of the application bt ping to the redis and the DB to
/// ensure that connection
async fn health(State(ctx): State<AppContext>) -> Result<Response> {
    #[cfg(feature = "with-db")]
    let is_db_ok = match ctx.db.ping().await {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(err.msg = %error, err.detail = ?error, "health_db_ping_error");
            false
        }
    };

    #[cfg(any(feature = "bg_pg", feature = "bg_redis", feature = "bg_sqlt"))]
    let is_queue_ok = {
        if let Some(queue) = ctx.queue_provider {
            match queue.ping().await {
                Ok(()) => true,
                Err(error) => {
                    tracing::error!(err.msg = %error, err.detail = ?error, "health_bg_redis_ping_error");
                    false
                }
            }
        } else {
            false
        }
    };

    #[cfg(feature = "cache_redis")]
    let is_redis_cache_ok = {
        match ctx.cache.driver.ping().await {
            // Reference: https://redis.io/docs/latest/commands/ping/#examples
            Ok(Some(result)) => result == "PONG",
            Ok(None) => false,
            Err(error) => {
                tracing::error!(err.msg = %error, err.detail = ?error, "health_cache_redis_ping_error");
                false
            }
        }
    };

    format::json(Health {
        #[cfg(feature = "with-db")]
        db: is_db_ok,
        #[cfg(any(feature = "bg_pg", feature = "bg_redis", feature = "bg_sqlt"))]
        queue: is_queue_ok,
        #[cfg(feature = "cache_redis")]
        redis_cache: is_redis_cache_ok,
    })
}

/// Defines and returns the health-related routes.
pub fn routes() -> Routes {
    Routes::new().add("/_health", get(health))
}
