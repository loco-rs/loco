//! # Application Error Handling

use std::panic::Location;

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

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{inner}\n{backtrace}")]
    WithBacktrace {
        inner: Box<Self>,
        backtrace: Box<std::backtrace::Backtrace>,
    },

    #[error("{0}")]
    Message(String),

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

    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),

    #[error("cannot parse `{1}`: {0}")]
    YAMLFile(#[source] serde_yaml::Error, String),

    #[error(transparent)]
    YAML(#[from] serde_yaml::Error),

    #[error(transparent)]
    EnvVar(#[from] std::env::VarError),

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

    #[error("{0}")]
    Hash(String),

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
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error(transparent)]
    InvalidHeaderName(#[from] InvalidHeaderName),

    #[error(transparent)]
    InvalidMethod(#[from] InvalidMethod),

    #[error(transparent)]
    TaskJoinError(#[from] tokio::task::JoinError),

    #[cfg(feature = "with-db")]
    // Model
    #[error(transparent)]
    Model(#[from] crate::model::ModelError),

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
    SemVer(#[from] semver::Error),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    Validation(#[from] ModelValidationErrors),

    #[error(transparent)]
    AxumFormRejection(#[from] axum::extract::rejection::FormRejection),
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

/// Provides a set of helper methods converting `Option<T>`s into [`Result<T>`](crate::Result)s.
pub trait LocoOptionExt<T> {
    /// Convert an option to an Error.
    ///
    /// Uses the typename to formulate an Error::Message
    ///
    /// ```rust
    /// # use loco_rs::prelude::*;
    /// let optional_foo: Option<i32> = None;
    /// let result: Result<i32> = optional_foo.dbg();
    /// let Err(Error::Message(msg)) = result else {
    ///     unreachable!();
    /// };
    ///
    /// assert_eq!(msg, "Found None::<i32> at src/errors.rs:7:40".to_string());
    /// ```
    #[track_caller]
    fn dbg(self) -> Result<T, Error>
    where
        T: std::any::Any;

    /// Convert an option to an Error with a custom [`Error::Message`].
    ///
    /// ```rust
    /// # use loco_rs::prelude::*;
    /// let optional_foo: Option<i32> = None;
    /// let result: Result<i32> = optional_foo.msg("Where'd my number go?");
    /// let Err(Error::Message(msg)) = result else {
    ///     unreachable!();
    /// };
    /// assert_eq!(msg, "Where'd my number go?".to_string())
    /// ```
    fn msg(self, msg: impl ToString) -> Result<T, Error>;

    /// Convert an option to an Error with an [`Error::CustomError`].
    ///
    /// ```rust
    /// # use loco_rs::prelude::*;
    /// # use axum::http::StatusCode;
    ///
    /// let optional_foo: Option<i32> = None;
    /// let result: Result<i32> = optional_foo.status(StatusCode::BAD_REQUEST, "Missing number", "Maybe don't set optional_foo to None");
    /// let Err(Error::CustomError(status, error_detail)) = result else {
    ///     unreachable!();
    /// };
    ///
    /// assert_eq!(status, StatusCode::BAD_REQUEST);
    /// assert_eq!(error_detail.error, Some("Missing number".to_string()));
    /// assert_eq!(error_detail.description, Some("Maybe don't set optional_foo to None".to_string()));
    /// ```
    fn status<T1: Into<String> + AsRef<str>, T2: Into<String> + AsRef<str>>(
        self,
        status: StatusCode,
        error: T1,
        description: T2,
    ) -> Result<T, Error>;
}

impl<T> LocoOptionExt<T> for Option<T> {
    #[track_caller]
    fn dbg(self) -> Result<T, Error>
    where
        T: std::any::Any,
    {
        match self {
            Some(val) => Ok(val),
            None => {
                let loc = Location::caller();
                let file = loc.file();
                let line = loc.line();
                let column = loc.column();
                let type_name = std::any::type_name::<T>();
                let val = "fo".to_string();
                val.contains("Found None::<i32> at ");
                Err(Error::Message(format!(
                    "Found None::<{type_name}> at {}:{}:{}",
                    file, line, column
                )))
            }
        }
    }
    fn msg(self, msg: impl ToString) -> Result<T, Error> {
        self.ok_or(Error::Message(msg.to_string()))
    }

    fn status<T1: Into<String> + AsRef<str>, T2: Into<String> + AsRef<str>>(
        self,
        status: StatusCode,
        error: T1,
        description: T2,
    ) -> Result<T, Error> {
        self.ok_or(Error::CustomError(
            status,
            ErrorDetail::new(error, description),
        ))
    }
}
