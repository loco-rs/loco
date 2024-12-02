use insta::assert_debug_snapshot;
use loco_rs::{config::OpenAPIType, prelude::*, tests_cfg};
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

trait OpenAPITrait {
    fn url(&self) -> &String;
}

impl OpenAPITrait for OpenAPIType {
    fn url(&self) -> &String {
        match self {
            OpenAPIType::Redoc { url, .. }
            | OpenAPIType::Scalar { url, .. }
            | OpenAPIType::Swagger { url, .. } => url,
        }
    }
}

#[rstest]
#[case("/redoc")]
#[case("/scalar")]
#[case("/swagger-ui")]
#[tokio::test]
#[serial]
async fn openapi(#[case] mut test_name: &str) {
    configure_insta!();

    let ctx: AppContext = tests_cfg::app::get_app_context().await;

    match test_name {
        "/redoc" => assert_eq!(ctx.config.server.openapi.redoc.url(), test_name),
        "/scalar" => assert_eq!(ctx.config.server.openapi.scalar.url(), test_name),
        _ => assert_eq!(ctx.config.server.openapi.swagger.url(), test_name),
    }

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    test_name = test_name.trim_start_matches("/");
    let res = reqwest::Client::new()
        .request(
            reqwest::Method::GET,
            infra_cfg::server::get_base_url() + test_name,
        )
        .send()
        .await
        .expect("valid response");

    assert_debug_snapshot!(
        format!("openapi_[{test_name}]"),
        (
            res.status().to_string(),
            res.url().to_string(),
            res.text()
                .await
                .unwrap()
                .lines()
                .find(|line| line.contains("<title>"))
                .and_then(|line| { line.split("<title>").nth(1)?.split("</title>").next() })
                .unwrap_or_default()
                .to_string(),
        )
    );

    handle.abort();
}

#[rstest]
#[case("redoc/openapi.json")]
#[case("scalar/openapi.json")]
#[case("api-docs/openapi.json")]
#[case("redoc/openapi.yaml")]
#[case("scalar/openapi.yaml")]
#[case("api-docs/openapi.yaml")]
#[tokio::test]
#[serial]
async fn openapi_spec(#[case] test_name: &str) {
    configure_insta!();

    let ctx: AppContext = tests_cfg::app::get_app_context().await;

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::Client::new()
        .request(
            reqwest::Method::GET,
            infra_cfg::server::get_base_url() + test_name,
        )
        .send()
        .await
        .expect("valid response");

    assert_debug_snapshot!(
        format!("openapi_spec_[{test_name}]"),
        (
            res.status().to_string(),
            res.url().to_string(),
            res.text().await.unwrap(),
        )
    );

    handle.abort();
}
