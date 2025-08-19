{% set mig_ts = ts | date(format="%Y%m%d_%H%M%S") -%}
{% set plural_snake = name | plural | snake_case -%}
{% set module_name = "m" ~  mig_ts ~ "_" ~ plural_snake -%}
{% set model = name | plural | pascal_case -%}
{% if with_tz %}
{% set create_table_func = "create_table" %}
{% else %}
{% set create_table_func = "create_table_without_timestamps" %}
{% endif %}
to: "migration/src/{{module_name}}.rs"
skip_glob: "migration/src/m????????_??????_{{plural_snake}}.rs"
message: "Migration for `{{name}}` added! You can now apply it with `$ cargo loco db migrate && cargo loco db entities`."
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
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        {{create_table_func}}(m, "{{plural_snake}}",
            &[
            {% if columns | length > 0 %}
            ("id", ColType::PkAuto),
            {% endif %}
            {% for column in columns -%}
            ("{{column.0}}", ColType::{{column.1}}),
            {% endfor -%}
            ],
            &[
            {% for ref in references -%}
            ("{{ref.0}}", "{{ref.1}}"),
            {% endfor -%}
            ]
        ).await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "{{plural_snake}}").await
    }
}
