use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto(UsersRoles::Table)
                    .primary_key(
                        Index::create()
                            .name("idx-users_roles-refs-pk")
                            .table(UsersRoles::Table)
                            .col(UsersRoles::UsersId)
                            .col(UsersRoles::RolesId),
                    )
                    .col(integer(UsersRoles::UsersId))
                    .col(integer(UsersRoles::RolesId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-users_roles-users")
                            .from(UsersRoles::Table, UsersRoles::UsersId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-users_roles-roles")
                            .from(UsersRoles::Table, UsersRoles::RolesId)
                            .to(Roles::Table, Roles::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UsersRoles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UsersRoles {
    Table,
    UsersId,
    RolesId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
#[derive(DeriveIden)]
enum Roles {
    Table,
    Id,
}
