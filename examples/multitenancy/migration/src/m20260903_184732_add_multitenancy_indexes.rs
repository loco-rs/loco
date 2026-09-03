use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_index(
            Index::create()
                .name("uidx-tenant-applications-tenant-application")
                .table(Alias::new("tenant_applications"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("application_id"))
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("uidx-roles-tenant-name")
                .table(Alias::new("roles"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("name"))
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("uidx-tenant-members-tenant-user")
                .table(Alias::new("tenant_members"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("user_id"))
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("uidx-tenant-member-roles-tenant-member-role")
                .table(Alias::new("tenant_member_roles"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("tenant_member_id"))
                .col(Alias::new("role_id"))
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("uidx-permissions-tenant-subscription-key")
                .table(Alias::new("permissions"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("tenant_application_id"))
                .col(Alias::new("key"))
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("uidx-role-permissions-tenant-role-permission")
                .table(Alias::new("role_permissions"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("role_id"))
                .col(Alias::new("permission_id"))
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-documents-tenant")
                .table(Alias::new("documents"))
                .col(Alias::new("tenant_id"))
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        for (name, table) in [
            ("idx-documents-tenant", "documents"),
            (
                "uidx-role-permissions-tenant-role-permission",
                "role_permissions",
            ),
            ("uidx-permissions-tenant-subscription-key", "permissions"),
            (
                "uidx-tenant-member-roles-tenant-member-role",
                "tenant_member_roles",
            ),
            ("uidx-tenant-members-tenant-user", "tenant_members"),
            ("uidx-roles-tenant-name", "roles"),
            (
                "uidx-tenant-applications-tenant-application",
                "tenant_applications",
            ),
        ] {
            m.drop_index(Index::drop().name(name).table(Alias::new(table)).to_owned())
                .await?;
        }
        Ok(())
    }
}
