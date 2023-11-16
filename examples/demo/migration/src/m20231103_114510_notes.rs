use std::borrow::BorrowMut;

use rustyrails::schema::*;
use sea_orm_migration::prelude::*;

use crate::m20220101_000001_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto(Notes::Table)
                    .col(pk_auto(Notes::Id).borrow_mut())
                    .col(uuid(Notes::Pid).borrow_mut())
                    .col(string_null(Notes::Title).borrow_mut())
                    .col(string_null(Notes::Content).borrow_mut())
                    .col(integer(Notes::OwnerId).borrow_mut())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-notes-users")
                            .from(Notes::Table, Notes::OwnerId)
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
            .drop_table(Table::drop().table(Notes::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Notes {
    Table,
    Id,
    Pid,
    Title,
    Content,
    OwnerId,
}
