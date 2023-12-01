use insta::assert_debug_snapshot;
use loco_rs::testing;
use loco_starter_template::app::App;
use serial_test::serial;

// TODO: see how to dedup / extract this to app-local test utils
// not to framework, because that would require a runtime dep on insta
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn can_get_index() {
    testing::request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/").await;

        assert_eq!(response.text(), "Loco");
        assert_eq!(response.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_print_echo() {
    configure_insta!();

    testing::request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/echo")
            .json(&serde_json::json!({"site": "Loco"}))
            .await;

        assert_debug_snapshot!((response.status_code(), response.text()));
    })
    .await;
}
