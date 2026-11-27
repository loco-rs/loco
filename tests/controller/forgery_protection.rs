//! `server.middlewares.forgery_protection` through a running server.

use axum::http::StatusCode;
use loco_rs::{
    controller::middleware::forgery_protection::{ForgeryProtection, Mode},
    prelude::*,
    tests_cfg,
};

use crate::infra_cfg;

async fn ok() -> Result<Response> {
    format::text("ok")
}

async fn start(
    config: ForgeryProtection,
    route: axum::routing::MethodRouter<AppContext>,
) -> (tokio::task::JoinHandle<()>, String) {
    start_with(config, route, |_| {}).await
}

async fn start_with(
    config: ForgeryProtection,
    route: axum::routing::MethodRouter<AppContext>,
    tweak: impl FnOnce(&mut AppContext),
) -> (tokio::task::JoinHandle<()>, String) {
    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;
    ctx.config.server.middlewares.forgery_protection = Some(config);
    tweak(&mut ctx);
    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", route, Some(port)).await;
    (handle, get_base_url_port(port))
}

fn header_mode() -> ForgeryProtection {
    ForgeryProtection {
        enable: true,
        mode: Mode::Header,
        ..Default::default()
    }
}

#[tokio::test]
async fn header_mode_refuses_cross_site_posts() {
    let (handle, base) = start(header_mode(), get(ok).post(ok)).await;
    let client = reqwest::Client::new();

    let post = |site: &'static str| client.post(&base).header("sec-fetch-site", site).send();
    assert_eq!(post("same-origin").await.unwrap().status(), StatusCode::OK);
    let refused = post("cross-site").await.unwrap();
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);
    assert!(refused.text().await.unwrap().contains("cross_site_request"));

    // A cross-site GET is a link click; it is never checked.
    let get = client
        .get(&base)
        .header("sec-fetch-site", "cross-site")
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);

    // Older browsers: Origin against Host.
    let foreign = client
        .post(&base)
        .header("origin", "https://evil.test")
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status(), StatusCode::FORBIDDEN);

    // No browser headers at all: curl, a script, a server.
    assert_eq!(
        client.post(&base).send().await.unwrap().status(),
        StatusCode::OK
    );

    handle.abort();
}

#[tokio::test]
async fn exempt_paths_are_not_checked() {
    let config = ForgeryProtection {
        exempt: vec!["/".to_string()],
        ..header_mode()
    };
    let (handle, base) = start(config, post(ok)).await;
    let res = reqwest::Client::new()
        .post(&base)
        .header("sec-fetch-site", "cross-site")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    handle.abort();
}

#[tokio::test]
async fn trusted_origins_may_post() {
    let config = ForgeryProtection {
        trusted_origins: vec!["https://admin.test".to_string()],
        ..header_mode()
    };
    let (handle, base) = start(config, post(ok)).await;
    let client = reqwest::Client::new();
    let post = |origin: &'static str| {
        client
            .post(&base)
            .header("sec-fetch-site", "cross-site")
            .header("origin", origin)
            .send()
    };
    // Origins compare without regard to host case or a default port.
    assert_eq!(
        post("https://ADMIN.test:443").await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        post("https://evil.test").await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    handle.abort();
}

#[tokio::test]
async fn disabled_by_default() {
    let (handle, base) = start(ForgeryProtection::default(), post(ok)).await;
    let res = reqwest::Client::new()
        .post(&base)
        .header("sec-fetch-site", "cross-site")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    handle.abort();
}

#[cfg(feature = "auth_session")]
mod token {
    use loco_rs::{
        config::{Auth, SessionConfig},
        controller::middleware::{
            forgery_protection::Middleware,
            limit_payload::{DefaultBodyLimitKind, LimitPayload},
            MiddlewareLayer,
        },
        environment::Environment,
    };
    use serde::Deserialize;

    use super::*;

    fn token_mode() -> ForgeryProtection {
        ForgeryProtection {
            enable: true,
            mode: Mode::Token,
            ..Default::default()
        }
    }

    /// Renders a form with the hidden field, the way a view would.
    async fn form() -> Result<Response> {
        let html = loco_rs::controller::views::template(
            r#"<form method="post">{{ csrf_field() }}</form>"#,
            serde_json::json!({}),
        )?;
        format::html(&html)
    }

    async fn from_handler(token: CsrfToken) -> Result<Response> {
        format::text(&token.masked())
    }

    #[derive(Deserialize)]
    struct Submit {
        name: String,
    }

    async fn submit(Form(params): Form<Submit>) -> Result<Response> {
        format::text(&params.name)
    }

    fn cookie_of(res: &reqwest::Response) -> String {
        let set = res
            .headers()
            .get("set-cookie")
            .expect("token cookie set")
            .to_str()
            .unwrap();
        set.split(';').next().unwrap().to_string()
    }

    fn token_of(html: &str) -> String {
        let start = html.find("value=\"").unwrap() + "value=\"".len();
        let end = html[start..].find('"').unwrap() + start;
        html[start..end].to_string()
    }

    #[tokio::test]
    async fn form_field_round_trip() {
        let (handle, base) = start(token_mode(), get(form).post(submit)).await;
        let client = reqwest::Client::new();

        let page = client.get(&base).send().await.unwrap();
        assert_eq!(page.status(), StatusCode::OK);
        let cookie = cookie_of(&page);
        assert!(cookie.starts_with("loco_csrf="));
        let html = page.text().await.unwrap();
        assert!(html.contains(r#"<input type="hidden" name="authenticity_token" value=""#));
        let token = token_of(&html);

        let body = format!("name=bob&authenticity_token={token}");
        let res = client
            .post(&base)
            .header("cookie", &cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        // The handler still reads the whole form.
        assert_eq!(res.text().await.unwrap(), "bob");

        // No token, a wrong token, or a token without its cookie.
        for (cookie, body) in [
            (cookie.as_str(), "name=bob".to_string()),
            (
                cookie.as_str(),
                "name=bob&authenticity_token=bogus".to_string(),
            ),
            ("", format!("name=bob&authenticity_token={token}")),
        ] {
            let res = client
                .post(&base)
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::FORBIDDEN);
            assert!(res
                .text()
                .await
                .unwrap()
                .contains("invalid_authenticity_token"));
        }

        handle.abort();
    }

    #[tokio::test]
    async fn header_and_extractor() {
        let (handle, base) = start(token_mode(), get(from_handler).post(ok)).await;
        let client = reqwest::Client::new();

        let page = client.get(&base).send().await.unwrap();
        let cookie = cookie_of(&page);
        let token = page.text().await.unwrap();

        let res = client
            .post(&base)
            .header("cookie", &cookie)
            .header("x-csrf-token", &token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        // An existing cookie is not sent again.
        assert!(res.headers().get("set-cookie").is_none());

        handle.abort();
    }

    #[tokio::test]
    async fn multipart_field() {
        let (handle, base) = start(token_mode(), get(form).post(ok)).await;
        let client = reqwest::Client::new();
        let page = client.get(&base).send().await.unwrap();
        let cookie = cookie_of(&page);
        let token = token_of(&page.text().await.unwrap());

        let boundary = "XyZ";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"authenticity_token\"\r\n\r\n{token}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\n\r\nhello\r\n--{boundary}--\r\n"
        );
        let res = client
            .post(&base)
            .header("cookie", &cookie)
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        handle.abort();
    }

    #[tokio::test]
    async fn pages_without_a_token_set_no_cookie() {
        let (handle, base) = start(token_mode(), get(ok)).await;
        let res = reqwest::get(&base).await.unwrap();
        assert!(res.headers().get("set-cookie").is_none());
        handle.abort();
    }

    /// A form page and the cookie that goes with it.
    async fn page(client: &reqwest::Client, base: &str) -> (String, String) {
        let page = client.get(base).send().await.unwrap();
        let cookie = cookie_of(&page);
        (cookie, token_of(&page.text().await.unwrap()))
    }

    #[tokio::test]
    async fn cross_site_is_refused_even_with_a_token() {
        let (handle, base) = start(token_mode(), get(form).post(ok)).await;
        let client = reqwest::Client::new();
        let (cookie, token) = page(&client, &base).await;

        let post = |site: &'static str| {
            client
                .post(&base)
                .header("cookie", &cookie)
                .header("x-csrf-token", &token)
                .header("sec-fetch-site", site)
                .send()
        };
        assert_eq!(post("same-origin").await.unwrap().status(), StatusCode::OK);
        // A sibling subdomain that planted its own cookie and token.
        for site in ["same-site", "cross-site"] {
            let res = post(site).await.unwrap();
            assert_eq!(res.status(), StatusCode::FORBIDDEN);
            assert!(res.text().await.unwrap().contains("cross_site_request"));
        }
        handle.abort();
    }

    #[tokio::test]
    async fn secure_cookies_get_the_host_prefix() {
        let (handle, base) = start_with(token_mode(), get(form).post(ok), |ctx| {
            ctx.config.server.host = "https://app.test".to_string();
        })
        .await;
        let client = reqwest::Client::new();
        let page = client.get(&base).send().await.unwrap();
        let set = page.headers()["set-cookie"].to_str().unwrap().to_string();
        assert!(set.starts_with("__Host-loco_csrf="), "{set}");
        assert!(set.contains("Secure"), "{set}");
        assert!(set.contains("Path=/"), "{set}");
        assert!(!set.contains("Domain"), "{set}");
        let cookie = cookie_of(&page);
        let token = token_of(&page.text().await.unwrap());

        let res = client
            .post(&base)
            .header("cookie", &cookie)
            .header("x-csrf-token", &token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        // The unprefixed name, as a sibling subdomain could set it, is ignored.
        let unprefixed = cookie.trim_start_matches("__Host-");
        let res = client
            .post(&base)
            .header("cookie", unprefixed)
            .header("x-csrf-token", &token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        handle.abort();
    }

    fn multipart(boundary: &str, token: &str, file: &[u8]) -> Vec<u8> {
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"authenticity_token\"\r\n\r\n{token}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.bin\"\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(file);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        body
    }

    #[tokio::test]
    async fn multipart_over_two_mib_within_the_app_limit() {
        let (handle, base) = start_with(token_mode(), get(form).post(ok), |ctx| {
            ctx.config.server.middlewares.limit_payload = Some(LimitPayload {
                body_limit: DefaultBodyLimitKind::Limit(10 * 1024 * 1024),
            });
        })
        .await;
        let client = reqwest::Client::new();
        let (cookie, token) = page(&client, &base).await;

        // The token comes before a 3 MiB file, past axum's 2 MiB multipart
        // default.
        let file = vec![b'a'; 3 * 1024 * 1024];
        let res = client
            .post(&base)
            .header("cookie", &cookie)
            .header("content-type", "multipart/form-data; boundary=XyZ")
            .body(multipart("XyZ", &token, &file))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        handle.abort();
    }

    #[tokio::test]
    async fn form_over_the_app_limit_is_413() {
        let (handle, base) = start_with(token_mode(), get(form).post(ok), |ctx| {
            ctx.config.server.middlewares.limit_payload = Some(LimitPayload {
                body_limit: DefaultBodyLimitKind::Limit(1024),
            });
        })
        .await;
        let client = reqwest::Client::new();
        let (cookie, token) = page(&client, &base).await;
        let body = format!("authenticity_token={token}&name={}", "a".repeat(4096));
        let res = client
            .post(&base)
            .header("cookie", &cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
        handle.abort();
    }

    #[tokio::test]
    async fn exempt_paths_need_no_token() {
        let config = ForgeryProtection {
            exempt: vec!["/".to_string()],
            ..token_mode()
        };
        let (handle, base) = start(config, post(ok)).await;
        let res = reqwest::Client::new()
            .post(&base)
            .header("sec-fetch-site", "cross-site")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        handle.abort();
    }

    async fn meta() -> Result<Response> {
        let html =
            loco_rs::controller::views::template("{{ csrf_meta_tags() }}", serde_json::json!({}))?;
        format::html(&html)
    }

    #[tokio::test]
    async fn meta_tags_carry_a_working_token() {
        let (handle, base) = start(token_mode(), get(meta).post(ok)).await;
        let client = reqwest::Client::new();
        let page = client.get(&base).send().await.unwrap();
        let cookie = cookie_of(&page);
        let html = page.text().await.unwrap();
        let prefix = r#"<meta name="csrf-param" content="authenticity_token"><meta name="csrf-token" content=""#;
        assert!(html.starts_with(prefix), "{html}");
        let token = html[prefix.len()..].trim_end_matches("\">");

        let res = client
            .post(&base)
            .header("cookie", &cookie)
            .header("x-csrf-token", token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        handle.abort();
    }

    #[tokio::test]
    async fn production_needs_the_session_secret() {
        let mut ctx: AppContext = tests_cfg::app::get_app_context().await;
        ctx.environment = Environment::Production;
        let err = Middleware::new(token_mode(), &ctx)
            .apply(axum::Router::new())
            .unwrap_err();
        assert!(err.to_string().contains("auth.session.secret"), "{err}");

        ctx.config.auth = Some(Auth {
            session: Some(SessionConfig {
                secret: Some("a production secret that is long enough".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert!(Middleware::new(token_mode(), &ctx)
            .apply(axum::Router::new())
            .is_ok());
    }
}
