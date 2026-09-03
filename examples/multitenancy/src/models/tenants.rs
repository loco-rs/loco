pub use super::_entities::tenants::{ActiveModel, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use sea_orm::DatabaseTransaction;
pub type Tenants = Entity;

use super::_entities::{
    applications, permissions, role_permissions, roles, tenant_applications, tenant_member_roles,
    tenant_members, tenants,
};

pub struct CreatedWorkspace {
    pub tenant: tenants::Model,
    pub application: applications::Model,
}

async fn create_role(
    txn: &DatabaseTransaction,
    tenant_id: i64,
    name: &str,
) -> ModelResult<roles::Model> {
    roles::ActiveModel {
        name: Set(name.to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(txn)
    .await
    .map_err(ModelError::from)
}

async fn create_permission(
    txn: &DatabaseTransaction,
    tenant_id: i64,
    tenant_application_id: i64,
    key: &str,
) -> ModelResult<permissions::Model> {
    permissions::ActiveModel {
        tenant_application_id: Set(tenant_application_id),
        key: Set(key.to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(txn)
    .await
    .map_err(ModelError::from)
}

async fn grant_permission(
    txn: &DatabaseTransaction,
    tenant_id: i64,
    role_id: i64,
    permission_id: i64,
) -> ModelResult<()> {
    role_permissions::ActiveModel {
        role_id: Set(role_id),
        permission_id: Set(permission_id),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(txn)
    .await?;
    Ok(())
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
    async fn create_workspace_in_transaction(
        txn: &DatabaseTransaction,
        user_id: i64,
        tenant_name: &str,
        tenant_slug: &str,
    ) -> ModelResult<CreatedWorkspace> {
        if tenants::Entity::find()
            .filter(tenants::Column::Slug.eq(tenant_slug))
            .one(txn)
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
        .insert(txn)
        .await?;

        let application = match applications::Entity::find()
            .filter(applications::Column::Name.eq("Documents"))
            .one(txn)
            .await?
        {
            Some(application) => application,
            None => {
                applications::ActiveModel {
                    name: Set("Documents".to_owned()),
                    ..Default::default()
                }
                .insert(txn)
                .await?
            }
        };

        let subscription = tenant_applications::ActiveModel {
            application_id: Set(application.id),
            status: Set("active".to_owned()),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(txn)
        .await?;

        let member = tenant_members::ActiveModel {
            user_id: Set(user_id),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(txn)
        .await?;

        let owner = create_role(txn, tenant.id, "Owner").await?;
        let manager = create_role(txn, tenant.id, "Manager").await?;
        let viewer = create_role(txn, tenant.id, "Viewer").await?;

        tenant_member_roles::ActiveModel {
            tenant_member_id: Set(member.id),
            role_id: Set(owner.id),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(txn)
        .await?;

        let read_permission =
            create_permission(txn, tenant.id, subscription.id, "documents:read").await?;
        let create_permission =
            create_permission(txn, tenant.id, subscription.id, "documents:create").await?;
        for (role_id, permission_id) in [
            (owner.id, read_permission.id),
            (owner.id, create_permission.id),
            (manager.id, read_permission.id),
            (manager.id, create_permission.id),
            (viewer.id, read_permission.id),
        ] {
            grant_permission(txn, tenant.id, role_id, permission_id).await?;
        }

        Ok(CreatedWorkspace {
            tenant,
            application,
        })
    }

    /// Creates a tenant workspace and assigns the user as its owner.
    ///
    /// # Errors
    ///
    /// Returns a model error when the slug exists or setup fails. The
    /// transaction is rolled back when any workspace record cannot be created.
    pub async fn create_workspace(
        db: &DatabaseConnection,
        user_id: i64,
        tenant_name: &str,
        tenant_slug: &str,
    ) -> ModelResult<CreatedWorkspace> {
        let txn = db.begin().await?;
        let workspace =
            Self::create_workspace_in_transaction(&txn, user_id, tenant_name, tenant_slug).await?;
        txn.commit().await?;
        Ok(workspace)
    }
}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
