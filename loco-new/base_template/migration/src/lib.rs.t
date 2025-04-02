#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

{%- if settings.auth %}
mod m20220101_000001_users;
{%- endif %}

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            {%- if settings.auth %}
            Box::new(m20220101_000001_users::Migration),
            {%- endif %}
            // inject-above (do not remove this comment)
        ]
    }
}
