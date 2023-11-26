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
    Cargo(#[from] cargo_metadata::Error),

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

    // TODO(review):. maybe change to to box instead expose all sidekiq errors
    #[error(transparent)]
    RedisPool(#[from] bb8::RunError<sidekiq::RedisError>),

    #[error(transparent)]
    Redis(#[from] sidekiq::redis_rs::RedisError),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}
