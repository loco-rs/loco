//! # Application Error Handling

use axum::http::StatusCode;
use config::ConfigError;
use lettre::{address::AddressError, transport::smtp};

use crate::controller::ErrorDetail;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error("task not found: '{0}'")]
    TaskNotFound(String),

    #[error(transparent)]
    Tera(#[from] tera::Error),

    #[error(transparent)]
    JSON(#[from] serde_json::Error),

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

    #[error(transparent)]
    Config(#[from] ConfigError),

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

    #[cfg(feature = "with-db")]
    // Model
    #[error(transparent)]
    Model(#[from] crate::model::ModelError),

    #[error(transparent)]
    RedisPool(#[from] bb8::RunError<sidekiq::RedisError>),

    #[error(transparent)]
    Redis(#[from] sidekiq::redis_rs::RedisError),

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
    pub fn string(s: &str) -> Self {
        Self::Message(s.to_string())
    }
    /*
    TODO: work on this when time allows
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
    */
}
