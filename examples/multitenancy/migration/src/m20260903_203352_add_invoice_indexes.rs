use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_index(
            Index::create()
                .name("uidx-invoices-tenant-subscription-number")
                .table(Alias::new("invoices"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("tenant_application_id"))
                .col(Alias::new("number"))
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx-invoices-tenant-subscription")
                .table(Alias::new("invoices"))
                .col(Alias::new("tenant_id"))
                .col(Alias::new("tenant_application_id"))
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        for name in [
            "idx-invoices-tenant-subscription",
            "uidx-invoices-tenant-subscription-number",
        ] {
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
