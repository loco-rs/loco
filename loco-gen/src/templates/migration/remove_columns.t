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
      {% for column in columns -%}
        manager
            .alter_table(
                alter({{tbl_enum}}::Table)
                  
                  .drop_column({{tbl_enum}}::{{column.0 | pascal_case}})
                  
                  .to_owned(),
            )
            .await?;
      {% endfor -%}
      Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
      {% for column in columns -%}
        manager
            .alter_table(
                alter({{tbl_enum}}::Table)
                  {% if column.1 == "decimal_len_null" or column.1 == "decimal_len" -%}
                  .add_column({{column.1}}({{tbl_enum}}::{{column.0 | pascal_case }}, 16, 4))
                  {% else -%}
                  .add_column({{column.1}}({{tbl_enum}}::{{column.0 | pascal_case}}))
                  {% endif -%}
                  .to_owned(),
            )
            .await?;
      {% endfor -%}
      Ok(())
    }
}

#[derive(DeriveIden)]
enum {{tbl_enum}} {
    Table,
    {% for column in columns -%}
    {{column.0 | pascal_case}},
    {% endfor %}
}
