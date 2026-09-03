use loco_rs::prelude::TenantQueryExt;
use loco_rs::testing::prelude::*;
use multitenancy::app::App;
use multitenancy::models::invoices;
use sea_orm::EntityTrait;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn seed_starts_without_invoices() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let invoices = invoices::Entity::find()
        .in_tenant(1)
        .all(&boot.app_context.db)
        .await
        .unwrap();

    assert!(invoices.is_empty());
}
