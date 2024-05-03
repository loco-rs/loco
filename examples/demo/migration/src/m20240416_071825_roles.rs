use sea_orm_migration::{prelude::*, schema::*};

use crate::{
    extension::postgres::Type,
    sea_orm::{DbBackend, DeriveActiveEnum, EnumIter, Schema},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create a new enum type `roles_name` with the values `Admin` and `User`
        let schema = Schema::new(DbBackend::Postgres);
        manager
            .create_type(schema.create_enum_from_active_enum::<RolesName>())
            .await?;
        // Create a new table `roles` with the columns `id`, `pid`, and `name`
        manager
            .create_table(
                table_auto(Roles::Table)
                    .col(pk_auto(Roles::Id))
                    .col(uuid_uniq(Roles::Pid))
                    .col(
                        ColumnDef::new(Roles::Name)
                            .custom(Alias::new("roles_name")) // Use the enum type name
                            .not_null()
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the table `roles`
        manager
            .drop_table(Table::drop().table(Roles::Table).to_owned())
            .await?;

        // Drop the enum type `roles_name`
        manager
            .drop_type(
                Type::drop()
                    .if_exists()
                    .name(RolesNameEnum)
                    .restrict()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Roles {
    Table,
    Id,
    Pid,
    Name,
}

// Create a new enum for the roles_name
#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "roles_name")]
pub enum RolesName {
    #[sea_orm(string_value = "Admin")]
    Admin,
    #[sea_orm(string_value = "User")]
    User,
}
