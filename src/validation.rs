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

#[cfg(feature = "with-db")]
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use validator::ValidationErrors;

// this is a line-serialization type. it is used as an intermediate format
// to hold validation error data when we transform from
// validation::ValidationErrors to DbErr and encode all information in json.
#[derive(Debug, Deserialize, Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ModelValidationMessage {
    pub code: String,
    pub message: Option<String>,
}

/// <DbErr conversion hack>
///
/// Convert `ModelValidationErrors` (pretty) into a `DbErr` (ugly) for database
/// handling.
///
/// Because `DbErr` is used in model hooks and we implement the hooks
/// in the trait, we MUST use `DbErr`, so we need to "hide" a _representation_
/// of the error in `DbErr::Custom`, so that it can be unpacked later down the
/// stream, in the central error response handler.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[error("Model validation failed")]
pub struct ModelValidationErrors {
    pub errors: BTreeMap<String, Vec<ValidationError>>,
}

impl From<ValidationErrors> for ModelValidationErrors {
    fn from(value: ValidationErrors) -> Self {
        let mut map: BTreeMap<String, Vec<ValidationError>> = BTreeMap::new();
        for (field, errs) in &value.field_errors() {
            let mut list: Vec<ValidationError> = Vec::with_capacity(errs.len());
            for err in *errs {
                let mut params: HashMap<String, serde_json::Value> = HashMap::new();
                for (k, v) in &err.params {
                    params.insert(k.to_string(), v.clone());
                }
                list.push(ValidationError {
                    code: err.code.to_string(),
                    message: err.message.as_ref().map(std::string::ToString::to_string),
                    params,
                });
            }
            map.insert((*field).to_string(), list);
        }
        Self { errors: map }
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
    let compact: BTreeMap<String, Vec<ModelValidationMessage>> = errors
        .errors
        .iter()
        .map(|(field, list)| {
            let flat: Vec<ModelValidationMessage> = list
                .iter()
                .map(|e| ModelValidationMessage {
                    code: e.code.clone(),
                    message: e.message.clone(),
                })
                .collect();
            (field.clone(), flat)
        })
        .collect();

    match serde_json::to_string(&compact) {
        Ok(s) => sea_orm::DbErr::Custom(s),
        Err(err) => sea_orm::DbErr::Custom(format!(
            "[before_save] could not parse validation errors. err: {err}"
        )),
    }
}

/// Implement `Validatable` for `ActiveModel` when you want it to have a
/// `validate()` function.
pub trait ValidatorTrait {
    /// Perform validation and return a normalized error type
    ///
    /// # Errors
    ///
    /// Returns `ModelValidationErrors` when validation fails.
    fn validate(&self) -> Result<(), ModelValidationErrors>;
}

/// Adapter: allow using the `validator` crate seamlessly
impl<T: validator::Validate> ValidatorTrait for T {
    fn validate(&self) -> Result<(), ModelValidationErrors> {
        validator::Validate::validate(self).map_err(ModelValidationErrors::from)
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
        let v = self.validator();
        validator::Validate::validate(&*v).map_err(ModelValidationErrors::from)
    }
    fn validator(&self) -> Box<dyn validator::Validate>;
}

#[cfg(test)]
mod tests {

    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use serde::Deserialize;
    use validator::Validate;

    use super::*;

    #[derive(Debug, Deserialize, Validate)]
    pub struct TestValidator {
        #[validate(length(min = 4, message = "Invalid min characters long."))]
        pub name: String,
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
            validator::Validate::validate(&data)
                .map_err(|e| into_db_error(&ModelValidationErrors::from(e)))
        );
    }

    // Custom validator example without the `validator` crate
    #[derive(Debug, Deserialize)]
    pub struct CustomValidator {
        pub name: String,
    }

    impl ValidatorTrait for CustomValidator {
        fn validate(&self) -> Result<(), ModelValidationErrors> {
            if self.name.len() < 4 {
                let mut errors: BTreeMap<String, Vec<ValidationError>> = BTreeMap::new();
                errors.insert(
                    "name".to_string(),
                    vec![ValidationError {
                        code: "length".to_string(),
                        message: Some("Invalid min characters long.".to_string()),
                        params: HashMap::new(),
                    }],
                );
                return Err(ModelValidationErrors { errors });
            }
            Ok(())
        }
    }

    #[rstest]
    #[case("ab")]
    #[case("abcd")]
    fn custom_validator_works(#[case] name: &str) {
        let v = CustomValidator {
            name: name.to_string(),
        };
        let res = v.validate();
        if name.len() < 4 {
            assert!(res.is_err());
        } else {
            assert!(res.is_ok());
        }
    }
}
