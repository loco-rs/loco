//! This module provides utility functions for handling validation errors for
//! structs. It useful if you want to validate model before inset to Database.
//!
//! # Example:
//!
//! In the following example you can see how you can validate a user model
//! ```rust,ignore
//! use loco_rs::{
//!    validation,
//!    validator::Validate,
//! };
//! use sea_orm::DbErr;
//! pub use myapp::_entities::users::ActiveModel;
//!
//! // Validation structure
//! #[derive(Debug, Validate, Deserialize)]
//! pub struct ModelValidator {
//!     #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
//!     pub name: String,
//! }
//!
//! /// Convert from UserModel to ModelValidator
//! impl From<&ActiveModel> for ModelValidator {
//!    fn from(value: &ActiveModel) -> Self {
//!        Self {
//!            name: value.name.as_ref().to_string(),
//!        }
//!    }
//! }
//!
//! /// Creating validator function
//! impl ActiveModel {
//!    pub fn validate(&self) -> Result<(), DbErr> {
//!        let validator: ModelValidator = self.into();
//!        validator.validate().map_err(validation::into_db_error)
//!    }
//! }
//!
//! /// Inheritance `before_save` function and run validation function to make sure that we are inset the expected data.
//! #[async_trait::async_trait]
//! impl ActiveModelBehavior for ActiveModel {
//!     async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
//!     where
//!         C: ConnectionTrait,
//!     {
//!         {
//!             self.validate()?;
//!             Ok(self)
//!         }
//!     }
//! }
//! ```
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use validator::{ValidationError, ValidationErrors};

#[derive(Debug, Deserialize, Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ModelValidation {
    pub code: String,
    pub message: Option<String>,
}

/// Validate the given email
///
/// # Errors
///
/// Return an error in case the email is invalid.
pub fn is_valid_email(email: &str) -> Result<(), ValidationError> {
    if email.contains('@') {
        Ok(())
    } else {
        Err(ValidationError::new("invalid email"))
    }
}

/// Convert `ValidationErrors` into a `HashMap` of field errors.
#[must_use]
pub fn into_errors(errors: &ValidationErrors) -> HashMap<String, Vec<ModelValidation>> {
    errors
        .field_errors()
        .iter()
        .map(|(field, field_errors)| {
            let errors = field_errors
                .iter()
                .map(|err| ModelValidation {
                    code: err.code.to_string(),
                    message: err.message.as_ref().map(std::string::ToString::to_string),
                })
                .collect();
            ((*field).to_string(), errors)
        })
        .collect()
}

/// Convert `ValidationErrors` into a JSON `Value`.
///
/// # Errors
/// when could not convert errors hashmap into a serde value
pub fn into_json_errors(
    errors: &ValidationErrors,
) -> Result<serde_json::Value, serde_json::error::Error> {
    let error_data = into_errors(errors);
    serde_json::to_value(error_data)
}

#[cfg(feature = "with-db")]
/// Convert `ValidationErrors` into a `DbErr` for database handling.
#[must_use]
pub fn into_db_error(errors: &ValidationErrors) -> sea_orm::DbErr {
    match into_json_errors(errors) {
        Ok(errors_json) => sea_orm::DbErr::Custom(errors_json.to_string()),
        Err(err) => sea_orm::DbErr::Custom(format!(
            "[before_save] could not parse validation errors. err: {err}"
        )),
    }
}

#[cfg(test)]
mod tests {

    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use serde::Deserialize;
    use validator::Validate;

    use super::*;

    #[derive(Debug, Validate, Deserialize)]
    pub struct TestValidator {
        #[validate(length(min = 4, message = "Invalid min characters long."))]
        pub name: String,
    }

    #[rstest]
    #[case("test@example.com", true)]
    #[case("invalid-email", false)]
    fn can_validate_email(#[case] test_name: &str, #[case] expected: bool) {
        assert_eq!(is_valid_email(test_name).is_ok(), expected);
    }

    #[cfg(feature = "with-db")]
    #[rstest]
    #[case("foo")]
    #[case("foo-bar")]
    fn can_validate_into_db_error(#[case] name: &str) {
        let data = TestValidator {
            name: name.to_string(),
        };

        assert_debug_snapshot!(
            format!("struct-[{name}]"),
            data.validate().map_err(|e| into_db_error(&e))
        );
    }
}
