use serde::{Deserialize, Serialize};

use crate::models::_entities::users;

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UserSession {
    pub token: String,
    pub user: UserDetail,
}
#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UserDetail {
    pub pid: String,
    pub email: String,
    pub name: String,
    pub last_login: String,
}

impl UserSession {
    #[must_use]
    pub fn new(user: &users::Model, token: &String) -> Self {
        Self {
            token: token.to_string(),
            user: UserDetail {
                pid: user.pid.to_string(),
                email: user.email.to_string(),
                name: user.name.to_string(),
                last_login: "n/a".to_string(),
            },
        }
    }
}
