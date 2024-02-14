use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto(Sessions::Table)
                    .col(pk_auto(Sessions::Id))
                    .col(string(Sessions::SessionId))
                    .col(timestamp(Sessions::ExpiresAt))
                    .col(integer(Sessions::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-sessions-users")
                            .from(Sessions::Table, Sessions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    Id,
    SessionId,
    ExpiresAt,
    UserId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
