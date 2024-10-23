use demo_app::app::App;
use insta::assert_debug_snapshot;
use loco_rs::{testing, tokio};
use rstest::rstest;
use serial_test::serial;
// TODO: see how to dedup / extract this to app-local test utils
// not to framework, because that would require a runtime dep on insta
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("view_engine");
        let _guard = settings.bind_to_scope();
    };
}

#[rstest]
#[case("home")]
#[case("hello")]
#[case("simple")]
#[tokio::test]
#[serial]
async fn can_get_view_engine(#[case] uri: &str) {
    configure_insta!();
    testing::request::<App, _, _>(|request, _ctx| async move {
        let response = request.get(&format!("/view-engine/{uri}")).await;

        assert_debug_snapshot!(
            uri.replace('/', "_"),
            (response.status_code(), response.text())
        );
    })
    .await;
}
