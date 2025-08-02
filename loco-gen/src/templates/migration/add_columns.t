{% set mig_ts = ts | date(format="%Y%m%d_%H%M%S") -%}
{% set mig_name = name | snake_case -%}
{% set plural_snake = table | plural | snake_case -%}
{% set module_name = "m" ~  mig_ts ~ "_" ~ mig_name -%}
to: "migration/src/{{module_name}}.rs"
skip_glob: "migration/src/m????????_??????_{{mig_name}}.rs"
message: "Migration `{{mig_name}}` added! You can now apply it with `$ cargo loco db migrate && cargo loco db entities`."
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
        {% for column in columns -%}
        add_column(m, "{{plural_snake}}", "{{column.0}}", ColType::{{column.1}}).await?;
        {% endfor -%}
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        {% for column in columns -%}
        remove_column(m, "{{plural_snake}}", "{{column.0}}").await?;
        {% endfor -%}
        Ok(())
    }
}
