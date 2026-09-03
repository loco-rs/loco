use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "clients",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("email", ColType::String),
            ],
            &[("tenant", "")],
        )
        .await?;
        m.create_index(
            Index::create()
                .name("uidx-clients-tenant-email")
                .table(Alias::new("clients"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("email"))
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "clients").await
    }
}
