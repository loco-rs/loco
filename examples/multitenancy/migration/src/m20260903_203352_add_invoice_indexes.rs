use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_index(
            Index::create()
                .name("uidx-invoices-tenant-number")
                .table(Alias::new("invoices"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("number"))
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-invoices-tenant")
                .table(Alias::new("invoices"))
                .col(Alias::new("tenant_id"))
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        for name in ["idx-invoices-tenant", "uidx-invoices-tenant-number"] {
            m.drop_index(
                Index::drop()
                    .name(name)
                    .table(Alias::new("invoices"))
                    .to_owned(),
            )
            .await?;
        }
        Ok(())
    }
}
