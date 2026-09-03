pub use super::_entities::tenants::{ActiveModel, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
pub type Tenants = Entity;

use super::{
    _entities::{
        applications, permissions, role_permissions, roles, tenant_applications,
        tenant_member_roles, tenant_members, tenants,
    },
    users,
};

pub struct RegisteredWorkspace {
    pub user: users::Model,
    pub tenant: tenants::Model,
    pub application: applications::Model,
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
    /// Atomically creates a user and an owner workspace with the Documents
    /// application and its default permissions.
    ///
    /// # Errors
    ///
    /// Returns a model error when any lookup, insert, or transaction operation
    /// fails. The transaction is rolled back when setup is incomplete.
    pub async fn register_workspace(
        db: &DatabaseConnection,
        user: &users::RegisterParams,
        tenant_name: &str,
        tenant_slug: &str,
    ) -> ModelResult<RegisteredWorkspace> {
        let txn = db.begin().await?;
        let user = users::Model::create_with_password_in_transaction(&txn, user).await?;

        if tenants::Entity::find()
            .filter(tenants::Column::Slug.eq(tenant_slug))
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists);
        }

        let tenant = tenants::ActiveModel {
            name: Set(tenant_name.to_owned()),
            slug: Set(tenant_slug.to_owned()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        let application = match applications::Entity::find()
            .filter(applications::Column::Name.eq("Documents"))
            .one(&txn)
            .await?
        {
            Some(application) => application,
            None => {
                applications::ActiveModel {
                    name: Set("Documents".to_owned()),
                    ..Default::default()
                }
                .insert(&txn)
                .await?
            }
        };

        let subscription = tenant_applications::ActiveModel {
            application_id: Set(application.id),
            status: Set("active".to_owned()),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(&txn)
        .await?;

        let member = tenant_members::ActiveModel {
            user_id: Set(user.id),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(&txn)
        .await?;

        let role = roles::ActiveModel {
            name: Set("Owner".to_owned()),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(&txn)
        .await?;

        tenant_member_roles::ActiveModel {
            tenant_member_id: Set(member.id),
            role_id: Set(role.id),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(&txn)
        .await?;

        for key in ["documents:read", "documents:create"] {
            let permission = permissions::ActiveModel {
                tenant_application_id: Set(subscription.id),
                key: Set(key.to_owned()),
                ..Default::default()
            }
            .set_tenant(tenant.id)?
            .insert(&txn)
            .await?;

            role_permissions::ActiveModel {
                role_id: Set(role.id),
                permission_id: Set(permission.id),
                ..Default::default()
            }
            .set_tenant(tenant.id)?
            .insert(&txn)
            .await?;
        }

        txn.commit().await?;

        Ok(RegisteredWorkspace {
            user,
            tenant,
            application,
        })
    }
}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
