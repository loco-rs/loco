use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "invoices",
            &[
                ("id", ColType::PkAuto),
                ("number", ColType::String),
                ("description", ColType::String),
                ("amount_cents", ColType::BigInteger),
                ("status", ColType::String),
            ],
            &[("tenant", "")],
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "invoices").await
    }
}
