//! # Application Error Handling

use axum::http::StatusCode;
use config::ConfigError;
use lettre::{address::AddressError, transport::smtp};

use crate::{controller::ErrorDetail, model::ModelError};

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
    Smtp(#[from] smtp::Error),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    DB(#[from] sea_orm::DbErr),

    #[error(transparent)]
    ParseAddress(#[from] AddressError),

    #[error(transparent)]
    Config(#[from] ConfigError),

    // API
    #[error("{0}")]
    Unauthorized(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("")]
    CustomError(StatusCode, ErrorDetail),

    // Model
    #[error(transparent)]
    Model(#[from] ModelError),

    // TODO(review):. maybe change to to box instead expose all sidekiq errors
    #[error(transparent)]
    RedisPool(#[from] bb8::RunError<sidekiq::RedisError>),

    #[error(transparent)]
    Redis(#[from] sidekiq::redis_rs::RedisError),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}
