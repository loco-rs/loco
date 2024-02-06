{% set mig_ts = ts | date(format="%Y%m%d_%H%M%S") -%}
{% set plural_snake = name | plural | snake_case -%}
{% set module_name = "m" ~  mig_ts ~ "_" ~ plural_snake -%}
{% set model = name | plural | pascal_case -%}
to: "migration/src/{{module_name}}.rs"
skip_glob: "migration/src/*_{{plural_snake}}.rs"
message: "Migration for `{{name}}` added! You can now apply it with `$ cargo loco db migrate`."
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
                    {% if is_link -%}
                    .primary_key(
                        Index::create()
                            .name("idx-{{plural_snake}}-refs-pk")
                            .table({{model}}::Table)
                            {% for ref in references -%}
                            .col({{model}}::{{ref.1 | pascal_case}})
                            {% endfor -%}
                            ,
                    )
                    {% else -%}
                    .col(pk_auto({{model}}::Id))
                    {% endif -%}
                    {% for column in columns -%}
                    .col({{column.1}}({{model}}::{{column.0 | pascal_case}}))
                    {% endfor -%}
                    {% for ref in references -%}
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-{{plural_snake}}-{{ref.0 | plural}}")
                            .from({{model}}::Table, {{model}}::{{ref.1 | pascal_case}})
                            .to({{ref.0 | plural | pascal_case}}::Table, {{ref.0 | plural | pascal_case}}::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    {% endfor -%}
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
    {% if is_link == false -%}
    Id,
    {% endif -%}
    {% for column in columns -%}
    {{column.0 | pascal_case}},
    {% endfor %}
}


{% for ref in references -%}
#[derive(DeriveIden)]
enum {{ref.0 | plural | pascal_case}} {
    Table,
    Id,
}
{% endfor -%}

