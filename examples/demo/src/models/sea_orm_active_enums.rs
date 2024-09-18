use std::fmt::Display;

pub use crate::models::_entities::sea_orm_active_enums::RolesName;

impl Display for RolesName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Admin => "Admin".to_string(),
            Self::User => "User".to_string(),
        };
        write!(f, "{}", str)
    }
}
