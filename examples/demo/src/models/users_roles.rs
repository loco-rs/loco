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
        // Find the user role if it exists
        let user_role = users_roles::Entity::find()
            .filter(Column::UsersId.eq(user.id.clone()))
            .one(db)
            .await?;
        // Update the user role if it exists, otherwise create it
        if let Some(mut user_role) = user_role {
            // Delete the user role if the role is different
            if user_role.roles_id == role.id {
                return Ok(user_role);
            }
            // Delete the user role, cannot update since it is a composite key
            user_role.delete(db).await?;
        }
        // Create the user role 
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
