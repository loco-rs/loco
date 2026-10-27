//! # Application Error Handling

use axum::{
    extract::rejection::JsonRejection,
    http::{
        header::{InvalidHeaderName, InvalidHeaderValue},
        method::InvalidMethod,
        StatusCode,
    },
};
#[cfg(feature = "mailer")]
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

    #[cfg(feature = "mailer")]
    #[error("Error sending email: '{0}'")]
    EmailSender(#[from] lettre::error::Error),

    #[cfg(feature = "mailer")]
    #[error("Error sending email (smtp): '{0}'")]
    Smtp(#[from] smtp::Error),

    #[error("Worker error: {0}")]
    Worker(String),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[cfg(feature = "with-db")]
    #[error(transparent)]
    DB(#[from] sea_orm::DbErr),

    #[cfg(feature = "mailer")]
    #[error(transparent)]
    ParseAddress(#[from] AddressError),

    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error(transparent)]
    InvalidHeaderName(#[from] InvalidHeaderName),

    #[error(transparent)]
    InvalidMethod(#[from] InvalidMethod),

    #[cfg(feature = "worker_redis")]
    #[error(transparent)]
    Redis(#[from] redis::RedisError),

    #[cfg(feature = "worker")]
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),

    #[error(transparent)]
    Cache(#[from] crate::cache::CacheError),

    #[cfg(all(debug_assertions, feature = "generators"))]
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

/// An error with a message of your own on top of the one that caused it.
/// Display shows the message; the cause stays reachable through `source()`.
#[derive(Debug)]
struct Contextual {
    msg: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl std::fmt::Display for Contextual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for Contextual {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Turn a foreign `Result` or `Option` into a [`crate::Result`] with a message
/// of your own, so `?` works without a hand-written `map_err`.
///
/// ```
/// use loco_rs::prelude::*;
///
/// fn port(raw: &str) -> Result<u16> {
///     raw.parse::<u16>().or_err("invalid port")
/// }
///
/// fn first(items: &[u8]) -> Result<u8> {
///     items.first().copied().or_err("no items")
/// }
///
/// assert_eq!(port("80").unwrap(), 80);
/// assert_eq!(port("x").unwrap_err().to_string(), "invalid port");
/// assert_eq!(first(&[]).unwrap_err().to_string(), "no items");
/// ```
///
/// For a `Result` the original error is kept as the `source()` of the new one,
/// so logs and `std::error::Error::source` walkers still see the whole chain.
/// The result is [`Error::Any`], which responds with a generic `500`; use
/// `Error::BadRequest` and friends when the caller should see the message.
///
/// The method is named `or_err` rather than `context` so it never collides
/// with `anyhow::Context` when both are in scope.
pub trait OrErr<T> {
    /// # Errors
    /// Returns [`Error::Any`] carrying `msg`, with the original error as its
    /// source when there is one.
    fn or_err(self, msg: impl Into<String>) -> crate::Result<T>;

    /// Like [`OrErr::or_err`], but the message is built only on failure.
    ///
    /// # Errors
    /// Same as [`OrErr::or_err`].
    fn or_err_with<S: Into<String>>(self, f: impl FnOnce() -> S) -> crate::Result<T>;
}

impl<T, E> OrErr<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn or_err(self, msg: impl Into<String>) -> crate::Result<T> {
        self.or_err_with(|| msg)
    }

    fn or_err_with<S: Into<String>>(self, f: impl FnOnce() -> S) -> crate::Result<T> {
        self.map_err(|source| {
            Error::wrap(Contextual {
                msg: f().into(),
                source: Some(Box::new(source)),
            })
        })
    }
}

impl<T> OrErr<T> for Option<T> {
    fn or_err(self, msg: impl Into<String>) -> crate::Result<T> {
        self.or_err_with(|| msg)
    }

    fn or_err_with<S: Into<String>>(self, f: impl FnOnce() -> S) -> crate::Result<T> {
        self.ok_or_else(|| {
            Error::wrap(Contextual {
                msg: f().into(),
                source: None,
            })
        })
    }
}

#[cfg(test)]
mod or_err_tests {

    use super::*;

    #[test]
    fn result_keeps_source_chain() {
        let err = "x".parse::<u8>().or_err("bad number").unwrap_err();
        assert_eq!(err.to_string(), "bad number");
        let Error::Any(inner) = &err else {
            panic!("expected Error::Any, got {err:?}")
        };
        let cause = inner.source().expect("source preserved");
        assert!(cause.is::<std::num::ParseIntError>());
    }

    #[test]
    fn option_none_has_message_and_no_source() {
        let err = None::<u8>.or_err("missing").unwrap_err();
        assert_eq!(err.to_string(), "missing");
        let Error::Any(inner) = &err else {
            panic!("expected Error::Any")
        };
        assert!(inner.source().is_none());
    }

    #[test]
    fn ok_values_pass_through_and_lazy_message_is_not_built() {
        assert_eq!("7".parse::<u8>().or_err("no").unwrap(), 7);
        assert_eq!(
            Some(1)
                .or_err_with(|| -> String { panic!("built eagerly") })
                .unwrap(),
            1
        );
    }

    #[test]
    fn lazy_message_on_failure() {
        let err = None::<u8>.or_err_with(|| format!("id {}", 3)).unwrap_err();
        assert_eq!(err.to_string(), "id 3");
    }
}
