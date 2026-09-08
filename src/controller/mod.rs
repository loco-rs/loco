//! Manage web server routing
//!
//! # Example
//!
//! This example you can adding custom routes into your application by
//! implementing routes trait from [`crate::app::Hooks`] and adding your
//! endpoints to your application
//!
//! ```rust
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

#[cfg(feature = "with-db")]
use crate::model::ModelError;
use crate::{errors::Error, Result};

mod app_routes;
mod backtrace;
mod describe;
pub mod extractor;
pub mod format;
pub mod middleware;
pub mod monitoring;
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
    pub fn new<T1: Into<String> + AsRef<str>, T2: Into<String> + AsRef<str>>(
        error: T1,
        description: T2,
    ) -> Self {
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

/// Build the `(StatusCode, ErrorDetail)` pair for a validation error, shared
/// by [`Error::Validation`] and <code>[Error::Model]([ModelError::Validation])</code>
/// so both report the same shape.
fn validation_error_response(
    errors: &crate::validation::ModelValidationErrors,
) -> (StatusCode, ErrorDetail) {
    (
        StatusCode::BAD_REQUEST,
        ErrorDetail {
            error: None,
            description: None,
            errors: Some(serde_json::to_value(&errors.errors).unwrap_or_default()),
        },
    )
}

impl IntoResponse for Error {
    /// Convert an `Error` into an HTTP response.
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
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
                // Delegate to the wrapped error's own response so the real HTTP
                // status is preserved (e.g. internal errors stay 500) instead of
                // being flattened to 400 whenever a backtrace was captured.
                return (*inner).into_response();
            }
            Self::BadRequest(err) => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("Bad Request", &err),
            ),
            Self::JsonRejection(err) => {
                tracing::debug!(err = err.body_text(), "json rejection");
                (err.status(), ErrorDetail::with_reason("Bad Request"))
            }
            Self::AxumFormRejection(err) => {
                tracing::debug!(err = err.body_text(), "form rejection");
                (err.status(), ErrorDetail::with_reason("Bad Request"))
            }

            Self::Validation(ref errors) => validation_error_response(errors),

            #[cfg(feature = "with-db")]
            Self::Model(ModelError::EntityNotFound) => (
                StatusCode::NOT_FOUND,
                ErrorDetail::new("not_found", "Resource was not found"),
            ),
            #[cfg(feature = "with-db")]
            Self::Model(ModelError::EntityAlreadyExists) => (
                StatusCode::CONFLICT,
                ErrorDetail::new("conflict", "Resource already exists"),
            ),
            #[cfg(feature = "multi-tenancy")]
            Self::Model(ModelError::TenantMismatch) => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("tenant_mismatch", "Model belongs to a different tenant"),
            ),
            #[cfg(feature = "with-db")]
            Self::Model(ModelError::Validation(ref errors)) => validation_error_response(errors),

            // --- Internal / infrastructure errors: deliberately no `_` arm.
            //
            // `Error` is `#[non_exhaustive]` for downstream crates, but this
            // match lives inside `loco_rs` itself, where an exhaustive match
            // over a local enum is allowed. Keeping it exhaustive (instead of
            // a trailing wildcard) means that adding a new `Error` variant
            // anywhere in the crate is a compile error here until someone
            // deliberately decides which HTTP status it should map to, rather
            // than silently defaulting to 500. Every variant below is an
            // internal/infrastructure error, so they are all mapped to the
            // same generic 500 response.
            Self::Message(_)
            | Self::InternalServerError
            | Self::QueueProviderMissing
            | Self::TaskNotFound(_)
            | Self::Scheduler(_)
            | Self::Axum(_)
            | Self::Tera(_)
            | Self::JSON(_)
            | Self::YAMLFile(_, _)
            | Self::YAML(_)
            | Self::EmailSender(_)
            | Self::Smtp(_)
            | Self::Worker(_)
            | Self::IO(_)
            | Self::ParseAddress(_)
            | Self::InvalidHeaderValue(_)
            | Self::InvalidHeaderName(_)
            | Self::InvalidMethod(_)
            | Self::Storage(_)
            | Self::Cache(_)
            | Self::VersionCheck(_)
            | Self::Any(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),

            #[cfg(feature = "with-db")]
            Self::DB(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),

            // Other `ModelError` variants are internal/infrastructure errors
            // (DB, generic `Any`, free-form `Message`, and, when `auth`
            // is enabled, `Jwt`) and are collapsed to 500 like their
            // top-level counterparts.
            #[cfg(feature = "with-db")]
            Self::Model(ModelError::DbErr(_) | ModelError::Any(_) | ModelError::Message(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),
            #[cfg(all(feature = "with-db", feature = "auth"))]
            Self::Model(ModelError::Jwt(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),

            #[cfg(feature = "worker_redis")]
            Self::Redis(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),

            #[cfg(feature = "worker")]
            Self::Sqlx(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),

            #[cfg(debug_assertions)]
            Self::Generators(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),
        };

        (public_facing_error.0, Json(public_facing_error.1)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    async fn response_json(err: Error) -> (StatusCode, serde_json::Value) {
        let response = err.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("failed to read response body");
        let json: serde_json::Value =
            serde_json::from_slice(&body).expect("response body is not valid JSON");
        (status, json)
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn model_entity_not_found_maps_to_404() {
        let (status, json) = response_json(Error::Model(ModelError::EntityNotFound)).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(
            json,
            serde_json::json!({
                "error": "not_found",
                "description": "Resource was not found"
            })
        );
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn model_entity_already_exists_maps_to_409() {
        let (status, json) = response_json(Error::Model(ModelError::EntityAlreadyExists)).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(
            json,
            serde_json::json!({
                "error": "conflict",
                "description": "Resource already exists"
            })
        );
    }

    #[cfg(feature = "multi-tenancy")]
    #[tokio::test]
    async fn model_tenant_mismatch_maps_to_400() {
        let (status, json) = response_json(Error::Model(ModelError::TenantMismatch)).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            json,
            serde_json::json!({
                "error": "tenant_mismatch",
                "description": "Model belongs to a different tenant"
            })
        );
    }

    #[cfg(feature = "with-db")]
    #[tokio::test]
    async fn model_validation_maps_same_as_top_level_validation() {
        use crate::validation::{ModelValidationErrors, ValidationError};
        use std::collections::BTreeMap;

        let mut errors: BTreeMap<String, Vec<ValidationError>> = BTreeMap::new();
        errors.insert(
            "username".to_string(),
            vec![ValidationError {
                code: "length".to_string(),
                message: Some("username must be at least 3 characters".to_string()),
                params: std::collections::HashMap::new(),
            }],
        );
        let model_errors = ModelValidationErrors {
            errors: errors.clone(),
        };

        let (model_status, model_json) =
            response_json(Error::Model(ModelError::Validation(model_errors))).await;
        let (top_level_status, top_level_json) =
            response_json(Error::Validation(ModelValidationErrors { errors })).await;

        assert_eq!(model_status, StatusCode::BAD_REQUEST);
        assert_eq!(model_status, top_level_status);
        assert_eq!(model_json, top_level_json);
    }

    #[tokio::test]
    async fn axum_form_rejection_maps_to_4xx_not_500() {
        #[derive(Debug, serde::Deserialize)]
        struct Data {
            #[allow(dead_code)]
            email: String,
        }

        let request = axum::http::Request::builder()
            .method(axum::http::Method::POST)
            .uri("/")
            .header(
                axum::http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(axum::body::Body::from(""))
            .unwrap();

        let rejection = axum::extract::Form::<Data>::from_request(request, &())
            .await
            .expect_err("expected a form rejection for a missing required field");

        let (status, json) = response_json(Error::AxumFormRejection(rejection)).await;

        assert!(status.is_client_error(), "expected 4xx, got {status}");
        assert_eq!(json, serde_json::json!({ "error": "Bad Request" }));
    }

    #[tokio::test]
    async fn not_found_still_maps_to_404() {
        let (status, json) = response_json(Error::NotFound).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(
            json,
            serde_json::json!({
                "error": "not_found",
                "description": "Resource was not found"
            })
        );
    }

    #[tokio::test]
    async fn infra_error_maps_to_500_with_standard_body() {
        for err in [
            Error::Message("boom".to_string()),
            Error::InternalServerError,
        ] {
            let (status, json) = response_json(err).await;

            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(
                json,
                serde_json::json!({
                    "error": "internal_server_error",
                    "description": "Internal Server Error"
                })
            );
        }
    }

    #[tokio::test]
    async fn bad_request_still_maps_to_400() {
        let (status, json) = response_json(Error::BadRequest("x".to_string())).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            json,
            serde_json::json!({
                "error": "Bad Request",
                "description": "x"
            })
        );
    }
}
