//! # Model Error Handling
//!
//! Useful when using `sea_orm` and want to propagate errors

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ModelValidation {
    pub code: String,
    pub message: Option<String>,
}

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum ModelError {
    #[error("Entity already exists")]
    EntityAlreadyExists,

    #[error("Entity not found")]
    EntityNotFound,

    #[error("{errors:?}")]
    ModelValidation { errors: ModelValidation },

    #[error("jwt error")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    DbErr(#[from] sea_orm::DbErr),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[allow(clippy::module_name_repetitions)]
pub type ModelResult<T, E = ModelError> = std::result::Result<T, E>;
