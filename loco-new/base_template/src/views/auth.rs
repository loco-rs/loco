use validators::auth::{CurrentResponse, LoginResponse};

use crate::models::_entities::users;

pub trait LoginResponseExt {
    fn new(user: &users::Model, token: &str) -> Self;
}

impl LoginResponseExt for LoginResponse {
    fn new(user: &users::Model, token: &str) -> Self {
        Self {
            token: token.to_string(),
            pid: user.pid.to_string(),
            name: user.name.clone(),
            is_verified: user.email_verified_at.is_some(),
        }
    }
}

pub trait CurrentResponseExt {
    fn new(user: &users::Model) -> Self;
}

impl CurrentResponseExt for CurrentResponse {
    fn new(user: &users::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
        }
    }
}
