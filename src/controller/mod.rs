//! Manage web server routing
//!
//! # Example
//!
//! This example you can adding custom routes into your application by
//! implementing routes trait from [`crate::app::Hooks`] and adding your
//! endpoints to your application
//!
//! ```rust, no_run
//! use async_trait::async_trait;
//! use loco_rs::{
//!    app::{AppContext, Hooks},
//!    boot::{create_app, BootResult, StartMode},
//!    config::Config,
//!    controller::AppRoutes,
//!    prelude::*,
//!    task::Tasks,
//!    environment::Environment,
//!    Result,
//! };
//! use sea_orm::DatabaseConnection;
//! use std::path::Path;
//! #[cfg(any(
//!     feature = "openapi_swagger",
//!     feature = "openapi_redoc",
//!     feature = "openapi_scalar"
//! ))]
//! use loco_rs::auth::openapi::{set_jwt_location_ctx, SecurityAddon};
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
//!     fn routes(ctx: &AppContext) -> AppRoutes {
//!         AppRoutes::with_default_routes()
//!             // .add_route(controllers::notes::routes())
//!     }
//!
//!     async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult>{
//!          create_app::<Self, Migrator>(mode, environment, config).await
//!     }
//!
//!     async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
//!         Ok(())
//!     }
//!
//!
//!     fn register_tasks(tasks: &mut Tasks) {}
//!
//!     async fn truncate(_ctx: &AppContext) -> Result<()> {
//!         Ok(())
//!     }
//!
//!     async fn seed(_ctx: &AppContext, base: &Path) -> Result<()> {
//!         Ok(())
//!     }
//!
//!     #[cfg(any(
//!         feature = "openapi_swagger",
//!         feature = "openapi_redoc",
//!         feature = "openapi_scalar"
//!     ))]
//!     fn inital_openapi_spec(ctx: &AppContext) -> utoipa::openapi::OpenApi {
//!         set_jwt_location_ctx(ctx);
//!
//!         #[derive(OpenApi)]
//!         #[openapi(
//!             modifiers(&SecurityAddon),
//!             info(
//!                 title = "Loco Demo",
//!                 description = "This app is a kitchensink for various capabilities and examples of the [Loco](https://loco.rs) project."
//!             )
//!         )]
//!         struct ApiDoc;
//!         set_jwt_location_ctx(ctx);
//!
//!         ApiDoc::openapi()
//!     }
//! }
//! ```

pub use app_routes::{AppRoutes, ListRoutes};
use axum::{
    extract::FromRequest,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use colored::Colorize;
pub use routes::Routes;
use serde::Serialize;

use crate::{errors::Error, Result};

mod app_routes;
mod backtrace;
mod describe;
pub mod extractor;
pub mod format;
#[cfg(feature = "with-db")]
mod health;
pub mod middleware;
#[cfg(any(
    feature = "openapi_swagger",
    feature = "openapi_redoc",
    feature = "openapi_scalar"
))]
mod openapi;
mod ping;
mod routes;
pub mod views;

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
/// use loco_rs::prelude::*;
///
/// async fn login() -> Result<Response> {
///     let valid = false;
///     if !valid {
///         return unauthorized("unauthorized access");
///     }
///     format::json(())
/// }
/// ````
pub fn unauthorized<T: Into<String>, U>(msg: T) -> Result<U> {
    Err(Error::Unauthorized(msg.into()))
}

/// Return a bad request with a message
///
/// # Errors
///
/// This function will return an error result
pub fn bad_request<T: Into<String>, U>(msg: T) -> Result<U> {
    Err(Error::BadRequest(msg.into()))
}

/// return not found status code
///
/// # Errors
/// Currently this function doesn't return any error. this is for feature
/// functionality
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
}

impl ErrorDetail {
    /// Create a new `ErrorDetail` with the specified error and description.
    #[must_use]
    pub fn new<T: Into<String> + AsRef<str>>(error: T, description: T) -> Self {
        let description = (!description.as_ref().is_empty()).then(|| description.into());
        Self {
            error: Some(error.into()),
            description,
            errors: None,
        }
    }

    /// Create an `ErrorDetail` with only an error reason and no description.
    #[must_use]
    pub fn with_reason<T: Into<String>>(error: T) -> Self {
        Self {
            error: Some(error.into()),
            description: None,
            errors: None,
        }
    }
}

#[derive(Debug, FromRequest)]
#[from_request(via(axum::Json), rejection(Error))]
pub struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

impl IntoResponse for Error {
    /// Convert an `Error` into an HTTP response.
    #[allow(clippy::cognitive_complexity)]
    fn into_response(self) -> Response {
        match &self {
            Self::WithBacktrace {
                inner,
                backtrace: _,
            } => {
                tracing::error!(
                error.msg = %inner,
                error.details = ?inner,
                "controller_error"
                );
            }
            err => {
                tracing::error!(
                error.msg = %err,
                error.details = ?err,
                "controller_error"
                );
            }
        }

        let public_facing_error = match self {
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
            Self::WithBacktrace { inner, backtrace } => {
                println!("\n{}", inner.to_string().red().underline());
                backtrace::print_backtrace(&backtrace).unwrap();
                (
                    StatusCode::BAD_REQUEST,
                    ErrorDetail::with_reason("Bad Request"),
                )
            }
            Self::BadRequest(err) => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("Bad Request", &err),
            ),
            Self::JsonRejection(err) => {
                tracing::debug!(err = err.body_text(), "json rejection");
                (err.status(), ErrorDetail::with_reason("Bad Request"))
            }

            Self::ValidationError(ref errors) => serde_json::to_value(errors).map_or_else(
                |_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ErrorDetail::new("internal_server_error", "Internal Server Error"),
                    )
                },
                |errors| {
                    (
                        StatusCode::BAD_REQUEST,
                        ErrorDetail {
                            error: None,
                            description: None,
                            errors: Some(errors),
                        },
                    )
                },
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),
        };

        (public_facing_error.0, Json(public_facing_error.1)).into_response()
    }
}
