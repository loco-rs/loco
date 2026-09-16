use serde::{Deserialize, Serialize};

use crate::models::_entities::users;

/// What a machine client is allowed to see about itself.
///
/// The entity also carries `password` and `api_key`; rendering the entity
/// directly would put both on the wire and would still compile.
#[derive(Debug, Deserialize, Serialize)]
pub struct MachineIdentity {
    pub pid: String,
    pub name: String,
}

impl From<&users::Model> for MachineIdentity {
    fn from(user: &users::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            name: user.name.clone(),
        }
    }
}
