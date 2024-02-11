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

use crate::controller::ErrorDetail;

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

    #[error("task not found: '{0}'")]
    TaskNotFound(String),

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

    #[error(transparent)]
    Smtp(#[from] smtp::Error),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[cfg(feature = "with-db")]
    #[error(transparent)]
    DB(#[from] sea_orm::DbErr),

    #[error(transparent)]
    RRgen(#[from] rrgen::Error),

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

    #[cfg(feature = "with-db")]
    // Model
    #[error(transparent)]
    Model(#[from] crate::model::ModelError),

    #[error(transparent)]
    RedisPool(#[from] bb8::RunError<sidekiq::RedisError>),

    #[error(transparent)]
    Redis(#[from] sidekiq::redis_rs::RedisError),

    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    Anyhow(#[from] eyre::Report),
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
    #[must_use]
    pub fn sources(&self) -> String {
        use std::{error::Error as _, fmt::Write as _};

        let mut chain = String::new();
        let mut source = self.source();
        while let Some(s) = source {
            let _ = writeln!(chain, "- {s}");
            source = s.source();
        }
        chain
    }
}
