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
//!    boot::{create_app, BootResult, StartMode},
//!    controller::{channels::AppChannels, AppRoutes},
//!    worker::Processor,
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
//!     async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult>{
//!          create_app::<Self, Migrator>(mode, environment).await
//!     }
//!     
//!    /// Only when `channels` feature is enabled
//!    fn register_channels(_ctx: &AppContext) -> AppChannels {
//!        let channels = AppChannels::default();
//!        //channels.register.ns("/", channels::application::on_connect);
//!        channels
//!    }
//!
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
#[cfg(feature = "channels")]
pub mod channels;
mod describe;
pub mod format;
#[cfg(feature = "with-db")]
mod health;
pub mod middleware;
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
/// use loco_rs::{
///     Result,
///     controller::{format, Json, unauthorized}
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
/// Currently this function did't return any error. this is for feature
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
}

impl ErrorDetail {
    /// Create a new `ErrorDetail` with the specified error and description.
    #[must_use]
    pub fn new<T: Into<String>>(error: T, description: T) -> Self {
        Self {
            error: Some(error.into()),
            description: Some(description.into()),
        }
    }

    /// Create an `ErrorDetail` with only an error reason and no description.
    #[must_use]
    pub fn with_reason<T: Into<String>>(error: T) -> Self {
        Self {
            error: Some(error.into()),
            description: None,
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

        match self {
            Self::NotFound => json_error_response(
                StatusCode::NOT_FOUND,
                ErrorDetail::new("not_found", "Resource was not found"),
            ),
            Self::InternalServerError => json_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),
            Self::Unauthorized(err) => {
                tracing::warn!(err);
                json_error_response(
                    StatusCode::UNAUTHORIZED,
                    ErrorDetail::new(
                        "unauthorized",
                        "You do not have permission to access this resource",
                    ),
                )
            }
            Self::CustomError(status_code, data) => json_error_response(status_code, data),
            Self::WithBacktrace { inner, backtrace } => {
                tracing::error!("\n{}", inner.to_string().red().underline());
                backtrace::print_backtrace(&backtrace).unwrap();
                json_error_response(
                    StatusCode::BAD_REQUEST,
                    ErrorDetail::with_reason("Bad Request"),
                )
            }
            Self::BadRequest(err) => json_error_response(
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("bad_request", &err),
            ),
            Self::Message(err) => (StatusCode::BAD_REQUEST, err).into_response(),
            _ => json_error_response(
                StatusCode::BAD_REQUEST,
                ErrorDetail::with_reason("Bad Request"),
            ),
        }
    }
}
/// Create a JSON error response with the specified status code and error
/// detail.
fn json_error_response(status_code: StatusCode, detail: ErrorDetail) -> Response {
    (status_code, Json(detail)).into_response()
}

#[cfg(test)]
mod tests {
    use futures_util::TryStreamExt;
    use serde::ser::{self};
    use serde_json::{json, Value};

    use super::*;

    async fn body_value(response: Response) -> Value {
        // Convert the body into a stream and collect the bytes
        let bytes = response
            .into_body()
            .into_data_stream()
            .map_ok(|bytes| bytes.to_vec())
            .try_concat()
            .await
            .unwrap_or_else(|_| Vec::new());

        // Convert the bytes to a String
        let body_string = String::from_utf8(bytes).unwrap();
        serde_json::from_str(&body_string).unwrap()
    }

    async fn body_string(response: Response) -> String {
        // Convert the body into a stream and collect the bytes
        let bytes = response
            .into_body()
            .into_data_stream()
            .map_ok(|bytes| bytes.to_vec())
            .try_concat()
            .await
            .unwrap_or_else(|_| Vec::new());

        // Convert the bytes to a String
        String::from_utf8(bytes).unwrap()
    }
    #[tokio::test]
    async fn test_unauthorized_method_error() {
        let result: Result<()> = unauthorized("unauthorized access");
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "unauthorized",
            "description":"You do not have permission to access this resource"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_bad_request_method_error() {
        let result: Result<()> = bad_request("bad request");
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "bad_request",
            "description":"bad request"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }
    #[tokio::test]
    async fn test_not_found_method_error() {
        let result: Result<()> = not_found();
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "not_found",
            "description":"Resource was not found"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_backtrace_error() {
        let result: Result<()> = Err(Error::WithBacktrace {
            inner: Box::new(Error::BadRequest("bad request".to_string())),
            backtrace: Box::new(std::backtrace::Backtrace::capture()),
        });
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "Bad Request"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_message_error() {
        let result: Result<()> = Err(Error::Message("bad request".to_string()));
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_string = body_string(response).await;

        // Compare the deserialized response body with the expected JSON
        assert_eq!(&body_string, "bad request");
    }

    #[tokio::test]
    async fn test_task_not_found_error() {
        let result: Result<()> = Err(Error::TaskNotFound("task not found".to_string()));
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error":"Bad Request"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_axum_error() {
        let axum_error = if let Err(e) = StatusCode::from_u16(6666) {
            let err = e.into();
            err
        } else {
            panic!("Bad status allowed!");
        };

        let result: Result<()> = Err(Error::Axum(axum_error));
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error":"Bad Request"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_tera_error() {
        let tera_error = tera::Error::msg("tera error");
        let result: Result<()> = Err(Error::Tera(tera_error));
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error":"Bad Request"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_json_error() {
        let json_error = ser::Error::custom("path contains invalid UTF-8 characters");
        let result: Result<()> = Err(Error::JSON(json_error));
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error":"Bad Request"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_unauthorized_error() {
        let result: Result<()> = Err(Error::Unauthorized("unauthorized access".to_string()));
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "unauthorized",
            "description":"You do not have permission to access this resource"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_not_found_error() {
        let result: Result<()> = Err(Error::NotFound);
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "not_found",
            "description":"Resource was not found"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_bad_request_error() {
        let result: Result<()> = Err(Error::BadRequest("bad request".to_string()));
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "bad_request",
            "description":"bad request"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_custom_error() {
        let result: Result<()> = Err(Error::CustomError(
            StatusCode::BAD_REQUEST,
            ErrorDetail::new("bad_request", "bad request"),
        ));
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "bad_request",
            "description":"bad request"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }

    #[tokio::test]
    async fn test_internal_server_error() {
        let result: Result<()> = Err(Error::InternalServerError);
        assert_eq!(result.is_err(), true);
        let response: Response = result.unwrap_err().into_response();
        // Status Code
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body_value = body_value(response).await;
        // Define the expected JSON value
        let expected = json!({
            "error": "internal_server_error",
            "description":"Internal Server Error"
        });

        // Compare the deserialized response body with the expected JSON
        assert_eq!(body_value, expected);
    }
}
