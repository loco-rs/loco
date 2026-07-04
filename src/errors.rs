//! # Application Error Handling

use axum::{
    extract::rejection::JsonRejection,
    http::{
        header::{InvalidHeaderName, InvalidHeaderValue},
        method::InvalidMethod,
        StatusCode,
    },
};
use lettre::{address::AddressError, transport::smtp};

use crate::{controller::ErrorDetail, depcheck, validation::ModelValidationErrors};

/*
backtrace principles:
- use a plan warapper variant with no 'from' conversion
- hand-code "From" conversion and force capture there with 'bt', which
  will wrap and create backtrace only if RUST_BACKTRACE=1.
costs:
- when RUST_BACKTRACE is not set, we don't pay for the capture and we dont pay for printing.

 */
impl From<serde_json::Error> for Error {
    fn from(val: serde_json::Error) -> Self {
        Self::JSON(val).bt()
    }
}

/// Application-wide error type.
///
/// Variants are grouped into two regions, in this fixed order:
///
/// 1. **Client-facing / API errors** — conditions that are meaningful to an
///    API caller and are mapped, one by one, to a specific status code by
///    `impl IntoResponse for Error` (see `controller::mod`). This includes
///    [`Error::Model`], since [`crate::model::ModelError`] variants
///    (`EntityNotFound`, `EntityAlreadyExists`, `Validation`) map to distinct
///    client statuses.
/// 2. **Internal / infrastructure errors** — everything else (I/O, DB,
///    queue, template rendering, email, etc.). These do not leak details to
///    callers and are deliberately collapsed to a generic `500 Internal
///    Server Error` by the same `IntoResponse` impl, via explicit (not
///    wildcard) match arms so the compiler forces every new variant to be
///    classified into one of the two regions.
///
/// This grouping is purely organizational: variant names, `#[error(...)]`
/// messages, `#[from]` conversions, and `#[cfg(...)]` gates are unchanged,
/// so it is not a breaking change.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    // ─────────────────────────── Client-facing / API errors ───────────────────────────
    #[error("{0}")]
    Message(String),

    // API
    #[error("{0}")]
    Unauthorized(String),

    // API
    #[error("not found")]
    NotFound,

    #[error("{0}")]
    BadRequest(String),

    #[error("")]
    CustomError(StatusCode, ErrorDetail),

    #[error("internal server error")]
    InternalServerError,

    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),

    #[error(transparent)]
    AxumFormRejection(#[from] axum::extract::rejection::FormRejection),

    #[error(transparent)]
    Validation(#[from] ModelValidationErrors),

    #[cfg(feature = "with-db")]
    // Model
    #[error(transparent)]
    Model(#[from] crate::model::ModelError),

    // ─────────────────────────── Internal / infrastructure errors ─────────────────────
    #[error("{inner}\n{backtrace}")]
    WithBacktrace {
        inner: Box<Self>,
        backtrace: Box<std::backtrace::Backtrace>,
    },

    #[error(
        "error while running worker: no queue provider populated in context. Did you configure \
         BackgroundQueue and connection details in `queue` in your config file?"
    )]
    QueueProviderMissing,

    #[error("task not found: '{0}'")]
    TaskNotFound(String),

    #[error(transparent)]
    Scheduler(#[from] crate::scheduler::Error),

    #[error(transparent)]
    Axum(#[from] axum::http::Error),

    #[error(transparent)]
    Tera(#[from] tera::Error),

    #[error(transparent)]
    JSON(serde_json::Error),

    #[error("cannot parse `{1}`: {0}")]
    YAMLFile(#[source] serde_yaml::Error, String),

    #[error(transparent)]
    YAML(#[from] serde_yaml::Error),

    #[error("Error sending email: '{0}'")]
    EmailSender(#[from] lettre::error::Error),

    #[error("Error sending email (smtp): '{0}'")]
    Smtp(#[from] smtp::Error),

    #[error("Worker error: {0}")]
    Worker(String),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[cfg(feature = "with-db")]
    #[error(transparent)]
    DB(#[from] sea_orm::DbErr),

    #[error(transparent)]
    ParseAddress(#[from] AddressError),

    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error(transparent)]
    InvalidHeaderName(#[from] InvalidHeaderName),

    #[error(transparent)]
    InvalidMethod(#[from] InvalidMethod),

    #[cfg(feature = "bg_redis")]
    #[error(transparent)]
    Redis(#[from] redis::RedisError),

    #[cfg(any(feature = "bg_pg", feature = "bg_sqlt"))]
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),

    #[error(transparent)]
    Cache(#[from] crate::cache::CacheError),

    #[cfg(debug_assertions)]
    #[error(transparent)]
    Generators(#[from] loco_gen::Error),

    #[error(transparent)]
    VersionCheck(#[from] depcheck::VersionCheckError),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    pub fn wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Any(Box::new(err)) //.bt()
    }

    pub fn msg(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Message(err.to_string()) //.bt()
    }
    #[must_use]
    pub fn string(s: &str) -> Self {
        Self::Message(s.to_string())
    }
    #[must_use]
    pub fn bt(self) -> Self {
        let backtrace = std::backtrace::Backtrace::capture();
        match backtrace.status() {
            std::backtrace::BacktraceStatus::Disabled
            | std::backtrace::BacktraceStatus::Unsupported => self,
            _ => Self::WithBacktrace {
                inner: Box::new(self),
                backtrace: Box::new(backtrace),
            },
        }
    }
}
