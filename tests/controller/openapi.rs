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
#[cfg_attr(feature = "openapi_swagger", case("/swagger-ui"))]
#[cfg_attr(feature = "openapi_redoc", case("/redoc"))]
#[cfg_attr(feature = "openapi_scalar", case("/scalar"))]
#[case("")]
#[tokio::test]
#[serial]
async fn openapi(#[case] mut test_name: &str) {
    if test_name.is_empty() {
        return;
    }
    configure_insta!();

    let ctx: AppContext = tests_cfg::app::get_app_context().await;

    match test_name {
        "/redoc" => {
            assert_eq!(
                ctx.config
                    .server
                    .openapi
                    .redoc
                    .clone()
                    .expect("redoc url is missing in test config")
                    .url(),
                test_name
            )
        }
        "/scalar" => assert_eq!(
            ctx.config
                .server
                .openapi
                .scalar
                .clone()
                .expect("scalar url is missing in test config")
                .url(),
            test_name
        ),
        _ => assert_eq!(
            ctx.config
                .server
                .openapi
                .swagger
                .clone()
                .expect("swagger url is missing in test config")
                .url(),
            test_name
        ),
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
#[cfg_attr(
    feature = "openapi_swagger",
    case("api-docs/openapi.json"),
    case("api-docs/openapi.yaml")
)]
#[cfg_attr(
    feature = "openapi_redoc",
    case("redoc/openapi.json"),
    case("redoc/openapi.yaml")
)]
#[cfg_attr(
    feature = "openapi_scalar",
    case("scalar/openapi.json"),
    case("scalar/openapi.yaml")
)]
#[case("")]
#[tokio::test]
#[serial]
async fn openapi_spec(#[case] test_name: &str) {
    if test_name.is_empty() {
        return;
    }
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
            res.headers().get("content-type").unwrap().to_owned(),
            res.text().await.unwrap(),
        )
    );

    handle.abort();
}
