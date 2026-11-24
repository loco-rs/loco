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
    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;
    ctx.config.server.middlewares.forgery_protection = Some(config);
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
}
