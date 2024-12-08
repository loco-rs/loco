use loco_rs::prelude::*;

pub use super::_entities::roles::{self, ActiveModel, Entity, Model};
use crate::models::{_entities::sea_orm_active_enums::RolesName, users, users_roles};

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            let mut this = self;
            this.pid = ActiveValue::Set(Uuid::new_v4());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

impl super::_entities::roles::Model {
    pub async fn add_user_to_admin_role(
        db: &DatabaseConnection,
        user: &users::Model,
    ) -> ModelResult<Self> {
        // Find the admin role
        let role = Self::upsert_by_name(db, RolesName::Admin).await?;
        // Connect the user to the admin role
        users_roles::Model::connect_user_to_role(db, user, &role).await?;
        Ok(role)
    }

    pub async fn add_user_to_user_role(
        db: &DatabaseConnection,
        user: &users::Model,
    ) -> ModelResult<Self> {
        // Find the user role
        let role = Self::upsert_by_name(db, RolesName::User).await?;
        // Connect the user to the user role
        users_roles::Model::connect_user_to_role(db, user, &role).await?;
        Ok(role)
    }

    pub async fn upsert_by_name(db: &DatabaseConnection, name: RolesName) -> ModelResult<Self> {
        let role = roles::Entity::find()
            .filter(roles::Column::Name.eq(name.clone()))
            .one(db)
            .await?;
        match role {
            Some(role) => Ok(role),
            None => {
                let role = roles::ActiveModel {
                    name: Set(name),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                Ok(role)
            }
        }
    }

    pub async fn find_by_user(db: &DatabaseConnection, user: &users::Model) -> ModelResult<Self> {
        let role = roles::Entity::find()
            .inner_join(users_roles::Entity)
            .filter(users_roles::Column::UsersId.eq(user.id))
            .one(db)
            .await?;
        role.ok_or_else(|| ModelError::EntityNotFound)
    }
}
