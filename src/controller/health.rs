//! This module contains a base routes related to health checks and status
//! reporting. These routes are commonly used to monitor the health of the
//! application and its dependencies.

use super::{format, routes::Routes};
use crate::controller::response::Health;
use crate::Result;
use axum::{response::Response, routing::get};

/// Check application ping endpoint
async fn health() -> Result<Response> {
    format::json(Health { ok: true })
}

/// Defines and returns the health-related routes.
pub fn routes() -> Routes {
    Routes::new().add("/_health", get(health))
}
