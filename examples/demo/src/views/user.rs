use serde::{Deserialize, Serialize};

use crate::models::{_entities::users, roles};

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentResponse {
    pub pid: String,
    pub name: String,
    pub email: String,
}

impl CurrentResponse {
    #[must_use]
    pub fn new(user: &users::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct UserResponse {
    pub pid: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

impl UserResponse {
    #[must_use]
    pub fn new(user: &users::Model, role: &roles::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
            role: role.name.clone().to_string(),
        }
    }
}
