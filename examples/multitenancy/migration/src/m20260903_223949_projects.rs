use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "projects",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("description", ColType::Text),
            ],
            &[("tenant", "")],
        )
        .await?;
        m.create_index(
            Index::create()
                .name("uidx-projects-tenant-name")
                .table(Alias::new("projects"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("name"))
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "projects").await
    }
}
