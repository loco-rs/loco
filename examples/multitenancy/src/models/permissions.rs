pub use super::_entities::permissions::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::{entity::prelude::*, JoinType, QuerySelect};
pub type Permissions = Entity;

use super::_entities::{
    permissions, role_permissions, roles, tenant_applications, tenant_member_roles, tenant_members,
};

impl TenantEntity for Entity {
    type TenantId = i64;

    fn tenant_column() -> Column {
        Column::TenantId
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

// implement your read-oriented logic here
impl Model {
    /// Returns whether a tenant member has `permission_key` for an active
    /// subscription to `application_id`.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::DbErr`] when the authorization query fails.
    pub async fn user_can(
        db: &DatabaseConnection,
        tenant_id: i64,
        user_id: i64,
        application_id: i64,
        permission_key: &str,
    ) -> ModelResult<bool> {
        Ok(permissions::Entity::find()
            .in_tenant(tenant_id)
            .filter(permissions::Column::Key.eq(permission_key))
            .join(
                JoinType::InnerJoin,
                permissions::Relation::TenantApplications.def(),
            )
            .filter(tenant_applications::Column::TenantId.eq(tenant_id))
            .filter(tenant_applications::Column::ApplicationId.eq(application_id))
            .filter(tenant_applications::Column::Status.eq("active"))
            .join(
                JoinType::InnerJoin,
                permissions::Relation::RolePermissions.def(),
            )
            .filter(role_permissions::Column::TenantId.eq(tenant_id))
            .join(JoinType::InnerJoin, role_permissions::Relation::Roles.def())
            .filter(roles::Column::TenantId.eq(tenant_id))
            .join(
                JoinType::InnerJoin,
                roles::Relation::TenantMemberRoles.def(),
            )
            .filter(tenant_member_roles::Column::TenantId.eq(tenant_id))
            .join(
                JoinType::InnerJoin,
                tenant_member_roles::Relation::TenantMembers.def(),
            )
            .filter(tenant_members::Column::TenantId.eq(tenant_id))
            .filter(tenant_members::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .is_some())
    }
}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
