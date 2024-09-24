use crate::infra_cfg;
use axum::http::StatusCode;
use insta::assert_debug_snapshot;
use loco_rs::{config, controller::middleware::remote_ip, prelude::*, tests_cfg};
use rstest::rstest;
use serial_test::serial;
use std::{collections::BTreeMap, path::PathBuf};

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
async fn middleware_panic(#[case] enable: bool) {
    configure_insta!();

    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        panic!("panic!")
    }

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;
    ctx.config.server.middlewares.catch_panic = Some(config::EnableMiddleware { enable });

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
async fn middleware_etag(#[case] enable: bool) {
    async fn action() -> Result<Response> {
        format::render().etag("loco-etag")?.text("content")
    }

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.etag = Some(config::EnableMiddleware { enable });

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
async fn middleware_remote_ip(#[case] enable: bool, #[case] expected: &str) {
    #[allow(clippy::items_after_statements)]
    async fn action(remote_ip: RemoteIP) -> Result<Response> {
        format::text(&remote_ip.to_string())
    }

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.remote_ip = Some(remote_ip::RemoteIPConfig {
        enable,
        trusted_proxies: Some(vec!["192.1.1.1/8".to_string()]),
    });

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
async fn middleware_timeout(#[case] enable: bool) {
    #[allow(clippy::items_after_statements)]
    async fn action() -> Result<Response> {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        format::render().text("loco")
    }

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.timeout_request =
        Some(config::TimeoutRequestMiddleware { enable, timeout: 2 });

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
async fn middleware_cors(
    #[case] enable: bool,
    #[case] test_name: &str,
    #[case] allow_headers: Option<Vec<String>>,
    #[case] allow_methods: Option<Vec<String>>,
    #[case] max_age: Option<u64>,
) {
    configure_insta!();

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.cors = Some(config::CorsMiddleware {
        enable,
        allow_origins: None,
        allow_headers,
        allow_methods,
        max_age,
    });

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::Client::new()
        .request(reqwest::Method::OPTIONS, infra_cfg::server::get_base_url())
        .send()
        .await
        .expect("valid response");

    assert_debug_snapshot!(
        format!("cors_[{test_name}]"),
        infra_cfg::response::get_headers_from_response(res)
    );

    handle.abort();
}

#[rstest]
#[case(true)]
#[case(false)]
#[tokio::test]
#[serial]
async fn middleware_limit_payload(#[case] enable: bool) {
    configure_insta!();

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.limit_payload = Some(config::LimitPayloadMiddleware {
        enable,
        body_limit: "1b".to_string(),
    });

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
async fn middleware_static_assets() {
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
    ctx.config.server.middlewares.static_assets = Some(config::StaticAssetsMiddleware {
        enable: true,
        must_exist: true,
        folder: config::FolderAssetsMiddleware {
            uri: "/static".to_string(),
            path: base_static_path.display().to_string(),
        },
        fallback: base_static_path.join("404.html").display().to_string(),
        precompressed: false,
    });

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
async fn middleware_secure_headers(
    #[case] preset: Option<String>,
    #[case] overrides: Option<BTreeMap<String, String>>,
) {
    configure_insta!();

    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;

    ctx.config.server.middlewares.secure_headers = Some(
        loco_rs::controller::middleware::secure_headers::SecureHeadersConfig {
            preset: preset.clone(),
            overrides: overrides.clone(),
        },
    );

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
// #[case(None, false, None)]
// #[case(Some(444), false, None)]
// #[case(None, true, None)]
#[case(None, false, Some("text fallback response".to_string()))]
#[tokio::test]
#[serial]
async fn middleware_fallback(
    #[case] code: Option<u16>,
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

    ctx.config.server.middlewares.fallback = Some(config::FallbackConfig {
        enable: true,
        code,
        file: file.clone().map(|f| f.display().to_string()),
        not_found: not_found.clone(),
    });

    let handle = infra_cfg::server::start_from_ctx(ctx).await;

    let res = reqwest::get(format!("{}not-found", infra_cfg::server::get_base_url()))
        .await
        .expect("valid response");

    if let Some(code) = code {
        assert_eq!(res.status(), code);
    } else if file.is_some() {
        assert_eq!(res.status(), StatusCode::OK);
    } else {
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
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
async fn middleware_powered_by_header(#[case] ident: Option<String>) {
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
