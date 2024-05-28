use blo::app::App;
use insta::assert_debug_snapshot;
use loco_rs::testing;

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
async fn ping() {
    configure_insta!();

    testing::request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("cache").await;
        assert_debug_snapshot!("key_not_exists", (response.text(), response.status_code()));
        let response = request.post("cache/insert").await;
        assert_debug_snapshot!("insert", (response.text(), response.status_code()));
        let response = request.get("cache").await;
        assert_debug_snapshot!("read_cache_key", (response.text(), response.status_code()));
    })
    .await;
}
