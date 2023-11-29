//! Manage web server routing
//!
//! # Example
//!
//! This example you can adding custom routes into your application by
//! implementing routes trait from [`crate::app::Hooks`] and adding your
//! endpoints to your application
//!
//! ```rust
//! # #[cfg(feature = "with-db")] {
//! use async_trait::async_trait;
//! use loco_rs::{
//!    app::{AppContext, Hooks},
//!    controller::AppRoutes,
//!    worker::Processor,
//!    task::Tasks,
//!    Result,
//! };
//! use sea_orm::DatabaseConnection;
//! use std::path::Path;
//!
//! /// this code block should be taken from the sea_orm migration model.
//! pub struct App;
//! pub use sea_orm_migration::prelude::*;
//! pub struct Migrator;
//! #[async_trait::async_trait]
//! impl MigratorTrait for Migrator {
//!     fn migrations() -> Vec<Box<dyn MigrationTrait>> {
//!         vec![]
//!     }
//! }
//!
//! #[async_trait]
//! impl Hooks for App {
//!
//!    fn app_name() -> &'static str {
//!        env!("CARGO_CRATE_NAME")
//!    }
//!
//!     fn routes() -> AppRoutes {
//!         AppRoutes::with_default_routes()
//!             // .add_route(controllers::notes::routes())
//!     }
//!
//!     fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {}
//!
//!     fn register_tasks(tasks: &mut Tasks) {}
//!
//!     async fn truncate(db: &DatabaseConnection) -> Result<()> {
//!         Ok(())
//!     }
//!
//!     async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {
//!         Ok(())
//!     }
//! }
//! }
//! ```

mod app_routes;
mod describe;
pub mod format;
#[cfg(feature = "with-db")]
mod health;
pub mod middleware;
mod ping;
mod routes;

pub use app_routes::{AppRoutes, ListRoutes};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
pub use routes::Routes;
use serde::Serialize;

use crate::{errors::Error, Result};

/// Create an unauthorized error with a specified message.
///
/// This function is used to generate an `Error::Unauthorized` variant with a
/// custom message.
///
/// # Errors
///
/// returns unauthorized enum
///
/// # Example
///
/// ```rust
/// use axum::Json;
/// use loco_rs::{
///     Result,
///     controller::{format, unauthorized}
/// };
///
/// async fn login() -> Result<Json<()>> {
///     let valid = false;
///     if !valid {
///         return unauthorized("unauthorized access");
///     }
///     format::json(())
/// }
/// ````
pub fn unauthorized<T>(msg: &str) -> Result<T> {
    Err(Error::Unauthorized(msg.to_string()))
}

pub fn not_found<T>() -> Result<T> {
    Err(Error::NotFound)
}
#[derive(Debug, Serialize)]
/// Structure representing details about an error.
pub struct ErrorDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ErrorDetail {
    /// Create a new `ErrorDetail` with the specified error and description.
    #[must_use]
    pub fn new(error: &str, description: &str) -> Self {
        Self {
            error: Some(error.to_string()),
            description: Some(description.to_string()),
        }
    }

    /// Create an `ErrorDetail` with only an error reason and no description.
    #[must_use]
    pub fn with_reason(error: &str) -> Self {
        Self {
            error: Some(error.to_string()),
            description: None,
        }
    }
}

impl IntoResponse for Error {
    /// Convert an `Error` into an HTTP response.
    fn into_response(self) -> Response {
        let (code, res) = match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                ErrorDetail::new("not_found", "Resource was not found"),
            ),
            Self::Unauthorized(err) => {
                tracing::warn!(err);
                (
                    StatusCode::UNAUTHORIZED,
                    ErrorDetail::new(
                        "unauthorized",
                        "You do not have permission to access this resource",
                    ),
                )
            }
            Self::CustomError(status_code, data) => (status_code, data),
            err => {
                tracing::warn!("{}", format!("{:?}", err));
                (
                    StatusCode::BAD_REQUEST,
                    ErrorDetail::with_reason("Bad Request"),
                )
            }
        };

        (code, Json(res)).into_response()
    }
}
