use insta::assert_debug_snapshot;
use loco_rs::{prelude::*, tests_cfg};
use rstest::rstest;
use serial_test::serial;

use crate::infra_cfg;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("openapi");
        let _guard = settings.bind_to_scope();
    };
}

#[rstest]
#[case("/redoc")]
#[case("/scalar")]
#[case("/swagger-ui")]
#[tokio::test]
#[serial]
async fn openapi(#[case] test_name: &str) {
    configure_insta!();

    let ctx: AppContext = tests_cfg::app::get_app_context().await;

    match test_name {
        "/redoc" => assert_eq!(ctx.config.server.openapi.redoc_url, test_name),
        "/scalar" => assert_eq!(ctx.config.server.openapi.scalar_url, test_name),
        _ => assert_eq!(ctx.config.server.openapi.swagger.swagger_url, test_name),
    }

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::Client::new()
        .request(
            reqwest::Method::OPTIONS,
            infra_cfg::server::get_base_url() + test_name,
        )
        .send()
        .await
        .expect("valid response")
        .text()
        .await
        .unwrap();

    assert_debug_snapshot!(format!("openapi_[{test_name}]"), res);

    handle.abort();
}
