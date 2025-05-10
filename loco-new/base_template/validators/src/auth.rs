use serde::{Deserialize, Serialize};
pub use validator::Validate;

#[derive(Default, Debug, Clone, Validate, Deserialize, Serialize)]
pub struct Validator {
    #[validate(length(min = 5, message = "Name must be at least 5 characters long."))]
    pub name: String,
    #[validate(email(message = "Invalid email address."))]
    pub email: String,
    #[validate(length(min = 25, message = "Password must be at least 25 characters long."))]
    pub password: String,
}

#[derive(Default, Debug, Clone, Validate, Deserialize, Serialize)]
pub struct RegisterParams {
    #[validate(length(min = 5, message = "Name must be at least 5 characters long."))]
    pub name: String,
    #[validate(email(message = "Invalid email address."))]
    pub email: String,
    #[validate(length(min = 25, message = "Password must be at least 25 characters long."))]
    pub password: String,
}

#[derive(Default, Debug, Clone, Validate, Deserialize, Serialize)]
pub struct LoginParams {
    #[validate(email(message = "Invalid email address."))]
    pub email: String,
    #[validate(length(min = 25, message = "Password must be at least 25 characters long."))]
    pub password: String,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub pid: String,
    pub name: String,
    pub is_verified: bool,
}

#[derive(Default, Debug, Clone, Validate, Deserialize, Serialize)]
pub struct ForgotParams {
    #[validate(email(message = "Invalid email address."))]
    pub email: String,
}

#[derive(Default, Debug, Clone, Validate, Deserialize, Serialize)]
pub struct ResetParams {
    pub token: String,
    #[validate(length(min = 25, message = "Password must be at least 25 characters long."))]
    pub password: String,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct CurrentResponse {
    pub pid: String,
    pub name: String,
    pub email: String,
}

#[derive(Default, Debug, Clone, Validate, Deserialize, Serialize)]
pub struct MagicLinkParams {
    #[validate(email(message = "Invalid email address."))]
    pub email: String,
}
