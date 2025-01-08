use demo_app::{app::App, models::users};
use insta::assert_debug_snapshot;
use loco_rs::testing::prelude::*;
use sea_orm::ModelTrait;
use serial_test::serial;

// TODO: see how to dedup / extract this to app-local test utils
// not to framework, because that would require a runtime dep on insta
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("cache");
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn ping() {
    configure_insta!();

    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("cache").await;
        assert_debug_snapshot!("key_not_exists", (response.text(), response.status_code()));
        let response = request.post("/cache/insert").await;
        assert_debug_snapshot!("insert", (response.text(), response.status_code()));
        let response = request.get("cache").await;
        assert_debug_snapshot!("read_cache_key", (response.text(), response.status_code()));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_or_insert() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let response = request.get("/cache/get_or_insert").await;
        assert_eq!(response.text(), "user1");

        let user = users::Model::find_by_email(&ctx.db, "user1@example.com")
            .await
            .unwrap();
        user.delete(&ctx.db).await.unwrap();
        let response = request.get("/cache/get_or_insert").await;
        assert_eq!(response.text(), "user1");
    })
    .await;
}
