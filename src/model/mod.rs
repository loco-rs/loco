//! # Model Error Handling
//!
//! Useful when using `sea_orm` and want to propagate errors

pub mod query;
use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::validation::ModelValidationErrors;

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum ModelError {
    #[error("Entity already exists")]
    EntityAlreadyExists,

    #[error("Entity not found")]
    EntityNotFound,

    #[error(transparent)]
    Validation(#[from] ModelValidationErrors),

    #[cfg(feature = "auth_jwt")]
    #[error("jwt error")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    DbErr(#[from] sea_orm::DbErr),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("{0}")]
    Message(String),
}

#[allow(clippy::module_name_repetitions)]
pub type ModelResult<T, E = ModelError> = std::result::Result<T, E>;

impl ModelError {
    #[must_use]
    pub fn wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Any(Box::new(err))
    }

    #[must_use]
    pub fn to_msg(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Message(err.to_string())
    }

    #[must_use]
    pub fn msg(s: &str) -> Self {
        Self::Message(s.to_string())
    }
}
#[async_trait]
pub trait Authenticable: Clone {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self>;
    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self>;
}
