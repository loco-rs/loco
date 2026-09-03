pub use super::_entities::tenants::{ActiveModel, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use sea_orm::DatabaseTransaction;
pub type Tenants = Entity;

pub const CORE_PERMISSION_KEYS: [&str; 11] = [
    "clients:view",
    "clients:create",
    "clients:edit",
    "projects:view",
    "projects:create",
    "projects:edit",
    "documents:view",
    "documents:create",
    "documents:edit",
    "billing:view",
    "billing:create",
];

use super::_entities::{
    applications, permissions, role_permissions, roles, tenant_applications, tenant_member_roles,
    tenant_members, tenants,
};

pub struct CreatedWorkspace {
    pub tenant: tenants::Model,
}

async fn find_or_create_application(
    txn: &DatabaseTransaction,
    name: &str,
) -> ModelResult<applications::Model> {
    if let Some(application) = applications::Entity::find()
        .filter(applications::Column::Name.eq(name))
        .one(txn)
        .await?
    {
        return Ok(application);
    }

    applications::ActiveModel {
        name: Set(name.to_owned()),
        ..Default::default()
    }
    .insert(txn)
    .await
    .map_err(ModelError::from)
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

async fn create_tenant_application(
    txn: &DatabaseTransaction,
    tenant_id: i64,
    application_id: i64,
    status: &str,
) -> ModelResult<tenant_applications::Model> {
    tenant_applications::ActiveModel {
        application_id: Set(application_id),
        status: Set(status.to_owned()),
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
    key: &str,
) -> ModelResult<permissions::Model> {
    permissions::ActiveModel {
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

fn role_has_default_permission(role: &str, permission: &str) -> bool {
    match role {
        "Owner" | "Administrator" => true,
        "Manager" => permission != "billing:create",
        "Support" => permission.ends_with(":view") && permission != "billing:view",
        _ => false,
    }
}

async fn provision_permissions(
    txn: &DatabaseTransaction,
    tenant_id: i64,
    tenant_roles: &[roles::Model],
) -> ModelResult<()> {
    for key in CORE_PERMISSION_KEYS {
        let permission = create_permission(txn, tenant_id, key).await?;
        for role in tenant_roles {
            if role_has_default_permission(&role.name, key) {
                grant_permission(txn, tenant_id, role.id, permission.id).await?;
            }
        }
    }
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

        for name in [
            "Analytics",
            "Client Portal",
            "Feature Flags",
            "Priority Support",
        ] {
            let addon = find_or_create_application(txn, name).await?;
            create_tenant_application(txn, tenant.id, addon.id, "inactive").await?;
        }

        let member = tenant_members::ActiveModel {
            user_id: Set(user_id),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(txn)
        .await?;

        let owner = create_role(txn, tenant.id, "Owner").await?;
        let administrator = create_role(txn, tenant.id, "Administrator").await?;
        let manager = create_role(txn, tenant.id, "Manager").await?;
        let support = create_role(txn, tenant.id, "Support").await?;

        tenant_member_roles::ActiveModel {
            tenant_member_id: Set(member.id),
            role_id: Set(owner.id),
            ..Default::default()
        }
        .set_tenant(tenant.id)?
        .insert(txn)
        .await?;

        provision_permissions(txn, tenant.id, &[owner, administrator, manager, support]).await?;

        Ok(CreatedWorkspace { tenant })
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

    /// Creates the default tenant-level permissions and role grants for fixtures.
    ///
    /// # Errors
    ///
    /// Returns a model error when roles cannot be loaded or grants cannot be inserted.
    pub async fn seed_access_defaults(db: &DatabaseConnection) -> ModelResult<()> {
        let txn = db.begin().await?;
        let seeded_tenants = tenants::Entity::find().all(&txn).await?;
        for tenant in seeded_tenants {
            let tenant_roles = roles::Entity::find().in_tenant(tenant.id).all(&txn).await?;
            provision_permissions(&txn, tenant.id, &tenant_roles).await?;
        }
        txn.commit().await?;
        Ok(())
    }
}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
