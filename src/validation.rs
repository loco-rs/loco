//! This module provides utility functions for handling validation errors for
//! structs. It useful if you want to validate model before insert to Database.
//!
//! # Example:
//!
//! In the following example you can see how you can validate a user model
//! ```rust,ignore
//! use loco_rs::prelude::*;
//! pub use myapp::_entities::users::ActiveModel;
//!
//! // Validation structure
//! #[derive(Debug, Validate, Deserialize)]
//! pub struct Validator {
//!     #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
//!     pub name: String,
//! }
//!
//! impl Validatable for ActiveModel {
//!   fn validator(&self) -> Box<dyn Validate> {
//!     Box::new(Validator {
//!         name: self.name.as_ref().to_owned(),
//!     })
//!   }
//! }
//!
//! /// Override `before_save` function and run validation to make sure that we insert valid data.
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

use std::ops::{Deref, DerefMut};

#[cfg(feature = "with-db")]
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError, ValidationErrors};

// this is a line-serialization type. it is used as an intermediate format
// to hold validation error data when we transform from
// validation::ValidationErrors to DbErr and encode all information in json.
#[derive(Debug, Deserialize, Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ModelValidationMessage {
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

///
/// <DbErr conversion hack>
///
/// Convert `ModelValidationErrors` (pretty) into a `DbErr` (ugly) for database
/// handling.
///
/// Because `DbErr` is used in model hooks and we implement the hooks
/// in the trait, we MUST use `DbErr`, so we need to "hide" a _representation_
/// of the error in `DbErr::Custom`, so that it can be unpacked later down the
/// stream, in the central error response handler.
#[derive(Debug, thiserror::Error)]
#[error("Model validation failed: {0}")]
pub struct ModelValidationErrors(ValidationErrors);

impl Deref for ModelValidationErrors {
    type Target = ValidationErrors;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ModelValidationErrors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "with-db")]
impl From<ModelValidationErrors> for DbErr {
    fn from(errors: ModelValidationErrors) -> Self {
        into_db_error(&errors)
    }
}

#[cfg(feature = "with-db")]
#[must_use]
pub fn into_db_error(errors: &ModelValidationErrors) -> sea_orm::DbErr {
    use std::collections::BTreeMap;

    let errors = &errors.0;
    let error_data: BTreeMap<String, Vec<ModelValidationMessage>> = errors
        .field_errors()
        .iter()
        .map(|(field, field_errors)| {
            let errors = field_errors
                .iter()
                .map(|err| ModelValidationMessage {
                    code: err.code.to_string(),
                    message: err.message.as_ref().map(std::string::ToString::to_string),
                })
                .collect();
            ((*field).to_string(), errors)
        })
        .collect();
    let json_errors = serde_json::to_value(error_data);
    match json_errors {
        Ok(errors_json) => sea_orm::DbErr::Custom(errors_json.to_string()),
        Err(err) => sea_orm::DbErr::Custom(format!(
            "[before_save] could not parse validation errors. err: {err}"
        )),
    }
}

/// Implement `Validatable` for `ActiveModel` when you want it to have a
/// `validate()` function.
pub trait Validatable {
    /// Perform validation
    ///
    /// # Errors
    ///
    /// This function will return an error if there are validation errors
    fn validate(&self) -> Result<(), ModelValidationErrors> {
        self.validator().validate().map_err(ModelValidationErrors)
    }
    fn validator(&self) -> Box<dyn Validate>;
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
            data.validate()
                .map_err(|e| into_db_error(&ModelValidationErrors(e)))
        );
    }
}
