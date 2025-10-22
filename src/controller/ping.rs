//! This module contains a base routes related to health checks and status
//! reporting. These routes are commonly used to monitor the health of the
//! application and its dependencies.

use aide::axum::{routing::get, IntoApiResponse};
use axum::Json;
use schemars::JsonSchema;
use serde::Serialize;

use super::routes::Routes;

/// Represents the health status of the application.
#[derive(Serialize, JsonSchema)]
struct Health {
    pub ok: bool,
}

/// Check application ping endpoint
async fn ping() -> impl IntoApiResponse {
    Json(Health { ok: true })
}

/// Defines and returns the health-related routes.
pub fn routes() -> Routes {
    Routes::new().add("/_ping", get(ping))
}
