use std::{collections::BTreeMap, path::PathBuf};

use axum::http::StatusCode;
use insta::assert_debug_snapshot;
use loco_rs::{controller::middleware, prelude::*, tests_cfg};
use rstest::rstest;
use serial_test::serial;

use crate::infra_cfg;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("middlewares");
        let _guard = settings.bind_to_scope();
    };
}

#[rstest]
#[case(true)]
#[case(false)]
#[tokio::test]
#[serial]
async fn panic(#[case] enable: bool) {
    configure_insta!();

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        panic!("panic!")
    }

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;
    ctx.config.server.middlewares.catch_panic = middleware::catch_panic::CatchPanic { enable };

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;
    let res = reqwest::get(infra_cfg::server::get_base_url()).await;

    if enable {
        let res = res.expect("valid response");
        assert_debug_snapshot!(
            format!("panic"),
            (res.status().to_string(), res.text().await)
        );
    } else {
        assert!(res.is_err());
    }

    handle.abort();
}

#[rstest]
#[case(true)]
#[case(false)]
#[tokio::test]
#[serial]
async fn etag(#[case] enable: bool) {
    async fn action() -> Result<Response> {
        format::render().etag("loco-etag")?.text("content")
    }

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.etag = middleware::etag::Etag { enable };

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;

    let res = reqwest::Client::new()
        .get(infra_cfg::server::get_base_url())
        .header("if-none-match", "loco-etag")
        .send()
        .await
        .expect("response");

    if enable {
        assert_eq!(res.status(), StatusCode::NOT_MODIFIED);
    } else {
        assert_eq!(res.status(), StatusCode::OK);
    }

    handle.abort();
}

#[rstest]
#[case(true, "remote: 51.50.51.50")]
#[case(false, "--")]
#[tokio::test]
#[serial]
async fn remote_ip(#[case] enable: bool, #[case] expected: &str) {
    #[allow(clippy::items_after_statements)]
    async fn action(remote_ip: RemoteIP) -> Result<Response> {
        format::text(&remote_ip.to_string())
    }

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.remote_ip = middleware::remote_ip::RemoteIpMiddleware {
        enable,
        trusted_proxies: Some(vec!["192.1.1.1/8".to_string()]),
    };

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;

    let res = reqwest::Client::new()
        .get(infra_cfg::server::get_base_url())
        .header(
            "x-forwarded-for",
            reqwest::header::HeaderValue::from_static("51.50.51.50,192.1.1.1"),
        )
        .send()
        .await
        .expect("response");

    assert_eq!(res.text().await.expect("string"), expected.to_string());

    handle.abort();
}

#[rstest]
#[case(true)]
#[case(false)]
#[tokio::test]
#[serial]
async fn timeout(#[case] enable: bool) {
    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        format::render().text("loco")
    }

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.timeout_request =
        middleware::timeout::TimeOut { enable, timeout: 2 };

    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action)).await;

    let res = reqwest::get(infra_cfg::server::get_base_url())
        .await
        .expect("response");

    if enable {
        assert_eq!(res.status(), StatusCode::REQUEST_TIMEOUT);
    } else {
        assert_eq!(res.status(), StatusCode::OK);
    }

    handle.abort();
}

#[rstest]
#[case(true, "default", None, None, None)]
#[case(true, "with_allow_headers", Some(vec!["token".to_string(), "user".to_string()]), None, None)]
#[case(true, "with_allow_methods", None, Some(vec!["post".to_string(), "get".to_string()]), None)]
#[case(true, "with_max_age", None, None, Some(20))]
#[case(false, "disabled", None, None, None)]
#[tokio::test]
#[serial]
async fn cors(
    #[case] enable: bool,
    #[case] test_name: &str,
    #[case] allow_headers: Option<Vec<String>>,
    #[case] allow_methods: Option<Vec<String>>,
    #[case] max_age: Option<u64>,
) {
    configure_insta!();

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    let mut middleware = loco_rs::controller::middleware::cors::Cors {
        enable,
        ..Default::default()
    };
    if let Some(allow_headers) = allow_headers {
        middleware.allow_headers = allow_headers;
    }
    if let Some(allow_methods) = allow_methods {
        middleware.allow_methods = allow_methods;
    }
    middleware.max_age = max_age;

    ctx.config.server.middlewares.cors = middleware;

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::Client::new()
        .request(reqwest::Method::OPTIONS, infra_cfg::server::get_base_url())
        .send()
        .await
        .expect("valid response");

    assert_debug_snapshot!(
        format!("cors_[{test_name}]"),
        (
            format!(
                "access-control-allow-origin: {:?}",
                res.headers().get("access-control-allow-origin")
            ),
            format!("vary: {:?}", res.headers().get("vary")),
            format!(
                "access-control-allow-methods: {:?}",
                res.headers().get("access-control-allow-methods")
            ),
            format!(
                "access-control-allow-headers: {:?}",
                res.headers().get("access-control-allow-headers")
            ),
            format!("allow: {:?}", res.headers().get("allow")),
        )
    );

    handle.abort();
}

#[rstest]
#[case(true)]
#[case(false)]
#[tokio::test]
#[serial]
async fn limit_payload(#[case] enable: bool) {
    configure_insta!();

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.limit_payload = middleware::limit_payload::LimitPayload {
        enable,
        body_limit: 0x1B,
    };

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::Client::new()
        .request(reqwest::Method::POST, infra_cfg::server::get_base_url())
        .body("send body".repeat(100))
        .send()
        .await
        .expect("valid response");

    if enable {
        assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
    } else {
        assert_eq!(res.status(), StatusCode::OK);
    }

    handle.abort();
}

#[tokio::test]
#[serial]
async fn static_assets() {
    configure_insta!();

    let base_static_assets_path = PathBuf::from("assets").join("static");
    let static_asset_path = tree_fs::Tree::default()
        .add(
            base_static_assets_path.join("404.html"),
            "<h1>404 not found</h1>",
        )
        .add(
            base_static_assets_path.join("static.html"),
            "<h1>static content</h1>",
        )
        .create()
        .expect("create static tree file");

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;
    let base_static_path = static_asset_path.join(base_static_assets_path);
    ctx.config.server.middlewares.static_assets = middleware::static_assets::StaticAssets {
        enable: true,
        must_exist: true,
        folder: middleware::static_assets::FolderConfig {
            uri: "/static".to_string(),
            path: base_static_path.display().to_string(),
        },
        fallback: base_static_path.join("404.html").display().to_string(),
        precompressed: false,
    };

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let get_static_html = reqwest::get("http://localhost:5555/static/static.html")
        .await
        .expect("valid response");

    assert_eq!(
        get_static_html.text().await.expect("text response"),
        "<h1>static content</h1>".to_string()
    );

    let get_fallback = reqwest::get("http://localhost:5555/static/logo.png")
        .await
        .expect("valid response");

    assert_eq!(
        get_fallback.text().await.expect("text response"),
        "<h1>404 not found</h1>".to_string()
    );

    handle.abort();
}

#[rstest]
#[case(None, None)]
#[case(Some("empty".to_string()), None)]
#[case(Some("github".to_string()), Some(BTreeMap::from([(
        "Content-Security-Policy".to_string(),
        "default-src 'self' https".to_string(),
    )])))]
#[tokio::test]
#[serial]
async fn secure_headers(
    #[case] preset: Option<String>,
    #[case] overrides: Option<BTreeMap<String, String>>,
) {
    configure_insta!();

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.secure_headers =
        loco_rs::controller::middleware::secure_headers::SecureHeader {
            enable: true,
            preset: preset.clone(),
            overrides: overrides.clone(),
        };

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::Client::new()
        .request(reqwest::Method::POST, infra_cfg::server::get_base_url())
        .send()
        .await
        .expect("response");

    let policy = res.headers().get("content-security-policy");
    let overrides_str = overrides.map_or("none".to_string(), |k| {
        k.keys()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    });
    assert_debug_snapshot!(
        format!(
            "secure_headers_[{}]_overrides[{}]",
            preset.unwrap_or_else(|| "none".to_string()),
            overrides_str
        ),
        policy
    );

    handle.abort();
}

#[rstest]
#[case(None, false, None)]
#[case(Some(StatusCode::BAD_REQUEST), false, None)]
#[case(None, true, None)]
#[case(None, false, Some("text fallback response".to_string()))]
#[tokio::test]
#[serial]
async fn fallback(
    #[case] code: Option<StatusCode>,
    #[case] file: bool,
    #[case] not_found: Option<String>,
) {
    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    let file = if file {
        Some(
            tree_fs::Tree::default()
                .add(
                    PathBuf::from("static_content.html"),
                    "<h1>fallback response</h1>",
                )
                .create()
                .unwrap()
                .join("static_content.html"),
        )
    } else {
        None
    };

    let mut fallback_config = middleware::fallback::Fallback {
        enable: true,
        file: file.clone().map(|f| f.display().to_string()),
        not_found: not_found.clone(),
        ..Default::default()
    };

    if let Some(code) = code {
        fallback_config.code = code;
    };

    ctx.config.server.middlewares.fallback = fallback_config;

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::get(format!("{}not-found", infra_cfg::server::get_base_url()))
        .await
        .expect("valid response");

    if let Some(code) = code {
        assert_eq!(res.status(), code);
    } else {
        assert_eq!(res.status(), StatusCode::OK);
    }

    let response_text = res.text().await.expect("response text");
    if file.is_some() {
        assert_eq!(response_text, "<h1>fallback response</h1>".to_string());
    }

    if let Some(not_found_text) = not_found {
        assert_eq!(response_text, not_found_text);
    }

    handle.abort();
}

#[rstest]
#[case(None)]
#[case(Some("custom".to_string()))]
#[tokio::test]
#[serial]
async fn powered_by_header(#[case] ident: Option<String>) {
    configure_insta!();

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.ident.clone_from(&ident);

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::get(infra_cfg::server::get_base_url())
        .await
        .expect("valid response");

    let header_value = res.headers().get("x-powered-by").expect("exists header");
    if let Some(ident_str) = ident {
        assert_eq!(header_value.to_str().expect("value"), ident_str);
    } else {
        assert_eq!(header_value.to_str().expect("value"), "loco.rs");
    }

    handle.abort();
}
