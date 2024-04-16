use loco_rs::prelude::*;
use sea_orm::{entity::prelude::*, ActiveValue};

pub use super::_entities::users_roles::{self, ActiveModel, Column, Entity, Model};

impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}

impl super::_entities::users_roles::Model {
    pub async fn connect_user_to_role(
        db: &DatabaseConnection,
        user: &super::users::Model,
        role: &super::roles::Model,
    ) -> ModelResult<Self> {
        let user_role = users_roles::ActiveModel {
            users_id: ActiveValue::set(user.id.clone()),
            roles_id: ActiveValue::set(role.id.clone()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(user_role)
    }
}
