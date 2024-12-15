{% set mig_ts = ts | date(format="%Y%m%d_%H%M%S") -%}
{% set mig_name = name | snake_case -%}
{% set tbl_enum = table | plural | pascal_case -%}
{% set module_name = "m" ~  mig_ts ~ "_" ~ mig_name -%}
to: "migration/src/{{module_name}}.rs"
skip_glob: "migration/src/m????????_??????_{{mig_name}}.rs"
message: "Migration `{{mig_name}}` added! You can now apply it with `$ cargo loco db migrate`."
injections:
- into: "migration/src/lib.rs"
  before: "inject-above"
  content: "            Box::new({{module_name}}::Migration),"
- into: "migration/src/lib.rs"
  before: "pub struct Migrator"
  content: "mod {{module_name}};"
---
use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                alter({{tbl_enum}}::Table)
                  {% for column in columns -%}
                  {% if column.1 == "decimal_len_null" or column.1 == "decimal_len" -%}
                  .add_column({{column.1}}({{tbl_enum}}::{{column.0 | pascal_case }}, 16, 4))
                  {% else -%}
                  .add_column({{column.1}}({{tbl_enum}}::{{column.0 | pascal_case}}))
                  {% endif -%}
                  {% endfor -%}
                  {% for ref in references -%}
                  .add_foreign_key(
                      TableForeignKey::new()
                          .name("fk-{{table | plural | snake_case}}-{{ref.0 | plural| snake_case}}")
                          .from_tbl({{tbl_enum}}::Table)
                          .from_col({{tbl_enum}}::{{ref.1 | pascal_case}})
                          .to_tbl({{ref.0 | plural | pascal_case}}::Table)
                          .to_col({{ref.0 | plural | pascal_case}}::Id)
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
            .alter_table(
              alter({{tbl_enum}}::Table)
                {% for ref in references -%}
                .drop_foreign_key(Alias::new("fk-{{table | plural | snake_case}}-{{ref.0 | plural| snake_case}}"))
                {% endfor -%}
                {% for column in columns -%}
                .drop_column({{tbl_enum}}::{{column.0 | pascal_case}})
                {% endfor -%}
                .to_owned()
            )
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

