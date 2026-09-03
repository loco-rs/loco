use loco_rs::prelude::TenantQueryExt;
use loco_rs::testing::prelude::*;
use multitenancy::app::App;
use multitenancy::models::invoices;
use sea_orm::EntityTrait;
use serial_test::serial;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn test_model() {
    configure_insta!();

    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let invoices = invoices::Entity::find()
        .in_tenant(1)
        .all(&boot.app_context.db)
        .await
        .unwrap();

    assert_eq!(invoices.len(), 2);
    assert!(invoices.iter().all(|invoice| invoice.tenant_id == 1));
}
