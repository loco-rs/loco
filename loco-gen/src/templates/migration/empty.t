{% set mig_ts = ts | date(format="%Y%m%d_%H%M%S") -%}
{% set mig_name = name | snake_case -%}
{% set module_name = "m" ~  mig_ts ~ "_" ~ mig_name -%}
to: "migration/src/{{module_name}}.rs"
skip_glob: "migration/src/*_{{mig_name}}.rs"
message: "Empty migration `{{mig_name}}` created — no schema change could be inferred from the name `{{name}}`, so its `up()` is unimplemented and `$ cargo loco db migrate` will panic until you write it. Names Loco does understand: CreateMovies, AddNameToUsers, RemoveNameFromUsers, AddUserRefToPosts, RenameTitleToNameOnMovies, CreateJoinTableUsersAndGroups."
injections:
- into: "migration/src/lib.rs"
  before: "inject-above"
  content: "            Box::new({{module_name}}::Migration),"
- into: "migration/src/lib.rs"
  before: "pub struct Migrator"
  content: "mod {{module_name}};"
---
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // Loco could not infer a schema change from the name `{{name}}`, so this
        // migration does nothing yet. Write the change here — the helpers in
        // `loco_rs::schema` (`add_column`, `remove_column`, `rename_column`,
        // `add_reference`, `create_table`, ...) cover the common cases — and
        // give `down()` the inverse. Until then `db migrate` panics here rather
        // than recording a migration that changed nothing.
        todo!("implement `up()` for the `{{name}}` migration")
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

