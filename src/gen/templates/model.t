{% set mig_ts = ts | date(format="%Y%m%d_%H%M%S") -%}
{% set plural_snake = name | plural | snake_case -%}
{% set module_name = "m" ~  mig_ts ~ "_" ~ plural_snake -%}
{% set model = name | plural | pascal_case -%}
to: "migration/src/{{module_name}}.rs"
skip_glob: "migration/src/*_{{plural_snake}}.rs"
message: "Migration for `{{name}}` added! You can now apply it with `$ rr db migrate`."
injections:
- into: "migration/src/lib.rs"
  before_last: "\\]"
  content: "            Box::new({{module_name}}::Migration),"
- into: "migration/src/lib.rs"
  before: "pub struct Migrator"
  content: "mod {{module_name}};"
---
use std::borrow::BorrowMut;

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto({{model}}::Table)
                    .col(pk_auto({{model}}::Id).borrow_mut())
                    .col(uuid({{model}}::Pid).borrow_mut())
                    .col(string_null({{model}}::Title).borrow_mut())
                    .col(string_null({{model}}::Content).borrow_mut())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table({{model}}::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum {{model}} {
    Table,
    Id,
    Pid,
    Title,
    Content,
}
