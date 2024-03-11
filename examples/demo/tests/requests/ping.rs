use blo::{app::App, models::_entities::notes::Entity};
use insta::{assert_debug_snapshot, with_settings};
use loco_rs::db::reset;
use loco_rs::testing;
use rstest::rstest;
use sea_orm::entity::prelude::*;
use serde_json;
use serial_test::serial;

// TODO: see how to dedup / extract this to app-local test utils
// not to framework, because that would require a runtime dep on insta
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("ping_request");
        let _guard = settings.bind_to_scope();
    };
}

// This tests the `_ping` endpoint, as well as the `NormalizePathLayer` that removes trailing
// slashes from the request path.
#[rstest]
#[case("ping", "/_ping")]
#[case("ping_with_trailing_slash", "/_ping/")]
#[case("ping_with_multiple_trailing_slashes", "/_ping////")]
#[tokio::test]
async fn ping(#[case] test_name: &str, #[case] path: &str) {
    configure_insta!();

    testing::request::<App, _, _>(|request, _ctx| async move {
        let response = request.get(path).await;

        assert_debug_snapshot!(test_name, (response.text(), response.status_code()));
    })
    .await;
}
