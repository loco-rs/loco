{% set mig_ts = ts | date(format="%Y%m%d_%H%M%S") -%}
{% set plural_snake = name | plural | snake_case -%}
{% set module_name = "m" ~  mig_ts ~ "_" ~ plural_snake -%}
{% set tbl_enum = table | plural | pascal_case -%}
to: "migration/src/{{module_name}}.rs"
skip_glob: "migration/src/m????????_??????_{{plural_snake}}.rs"
message: "Migration for `{{name}}` added! You can now apply it with `$ cargo loco db migrate`."
injections:
- into: "migration/src/lib.rs"
  before: "inject-above"
  content: "            Box::new({{module_name}}::Migration),"
- into: "migration/src/lib.rs"
  before: "pub struct Migrator"
  content: "mod {{module_name}};"
---
use loco_rs::schema::table_auto_tz;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto_tz({{tbl_enum}}::Table)
                    .primary_key(
                        Index::create()
                            .name("idx-{{plural_snake}}-refs-pk")
                            .table({{tbl_enum}}::Table)
                            {% for ref in references -%}
                            .col({{tbl_enum}}::{{ref.1 | pascal_case}})
                            {% endfor -%}
                            ,
                    )
                    {% for column in columns -%}
                    {% if column.1 == "decimal_len_null" or column.1 == "decimal_len" -%}
                    .col({{column.1}}({{tbl_enum}}::{{column.0 | pascal_case }}, 16, 4))
                    {% else -%}
                    .col({{column.1}}({{tbl_enum}}::{{column.0 | pascal_case}}))
                    {% endif -%}
                    {% endfor -%}
                    {% for ref in references -%}
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-{{plural_snake}}-{{ref.1 | plural| snake_case}}")
                            .from({{tbl_enum}}::Table, {{tbl_enum}}::{{ref.1 | pascal_case}})
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
            .drop_table(Table::drop().table({{tbl_enum}}::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum {{tbl_enum}} {
    Table,
    {% for column in columns -%}
    {{column.0 | pascal_case}},
    {% endfor %}
}

{% for ref in references | unique(attribute="0") -%}
#[derive(DeriveIden)]
enum {{ref.0 | plural | pascal_case}} {
    Table,
    Id,
}
{% endfor -%}

