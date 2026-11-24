use loco_rs::{
    boot,
    config::Webhook,
    controller::{
        extractor::webhook::{sign, SignedWebhook, WebhookSource},
        format, AppRoutes,
    },
    prelude::*,
    tests_cfg::{self, db::AppHook},
};
use serde::Deserialize;

use crate::infra_cfg;

struct GitHub;
impl WebhookSource for GitHub {
    const NAME: &'static str = "github";
}

struct Stripe;
impl WebhookSource for Stripe {
    const NAME: &'static str = "stripe";
}

struct Unconfigured;
impl WebhookSource for Unconfigured {
    const NAME: &'static str = "missing";
}

#[derive(Deserialize)]
struct Event {
    action: String,
}

async fn github(hook: SignedWebhook<GitHub>) -> Result<Response> {
    let event: Event = hook.json()?;
    format::text(&format!(
        "{} {}",
        hook.header("X-GitHub-Event").unwrap_or_default(),
        event.action
    ))
}

async fn stripe(hook: SignedWebhook<Stripe>) -> Result<Response> {
    format::text(&hook.body().len().to_string())
}

async fn unconfigured(_hook: SignedWebhook<Unconfigured>) -> Result<Response> {
    format::empty()
}

fn webhook(json: serde_json::Value) -> Webhook {
    serde_json::from_value(json).expect("webhook config")
}

struct Server {
    port: i32,
    handle: tokio::task::JoinHandle<()>,
    github: Webhook,
    stripe: Webhook,
}

async fn serve() -> Server {
    let mut ctx: AppContext = tests_cfg::app::get_app_context().await;
    let github_cfg = webhook(serde_json::json!({ "secret": "gh-secret" }));
    let stripe_cfg = webhook(serde_json::json!({ "secret": "whsec", "scheme": "timestamped" }));
    ctx.config
        .webhooks
        .insert("github".to_string(), github_cfg.clone());
    ctx.config
        .webhooks
        .insert("stripe".to_string(), stripe_cfg.clone());

    let router = AppRoutes::empty()
        .add_route(
            Routes::new()
                .prefix("/webhooks")
                .add("/github", post(github))
                .add("/stripe", post(stripe))
                .add("/missing", post(unconfigured)),
        )
        .to_router::<AppHook>(ctx.clone(), axum::Router::new())
        .expect("to router");

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_from_boot(
        boot::BootResult {
            app_context: ctx,
            router: Some(router),
            worker: None,
            run_scheduler: false,
        },
        Some(port),
    )
    .await;
    Server {
        port,
        handle,
        github: github_cfg,
        stripe: stripe_cfg,
    }
}

async fn post_signed(
    port: i32,
    path: &str,
    body: &'static str,
    headers: Vec<(String, String)>,
) -> reqwest::Response {
    let mut req = reqwest::Client::new()
        .post(format!("{}webhooks/{path}", get_base_url_port(port)))
        .header("X-GitHub-Event", "pull_request")
        .body(body);
    for (name, value) in headers {
        req = req.header(name, value);
    }
    req.send().await.expect("request")
}

const BODY: &str = r#"{"action":"opened"}"#;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[tokio::test]
async fn verified_github_webhook_reaches_the_handler() {
    let server = serve().await;

    let headers = sign(&server.github, BODY.as_bytes(), now());
    let res = post_signed(server.port, "github", BODY, headers).await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "pull_request opened");

    server.handle.abort();
}

#[tokio::test]
async fn bad_github_webhooks_are_refused() {
    let server = serve().await;

    // No signature.
    let res = post_signed(server.port, "github", BODY, vec![]).await;
    assert_eq!(res.status(), 401);

    // Signed with another secret.
    let other = webhook(serde_json::json!({ "secret": "nope" }));
    let res = post_signed(
        server.port,
        "github",
        BODY,
        sign(&other, BODY.as_bytes(), now()),
    )
    .await;
    assert_eq!(res.status(), 401);

    // Body changed after signing.
    let headers = sign(&server.github, BODY.as_bytes(), now());
    let res = post_signed(server.port, "github", r#"{"action":"closed"}"#, headers).await;
    assert_eq!(res.status(), 401);

    // Wrong prefix.
    let headers = vec![("X-Hub-Signature-256".to_string(), "md5=abc".to_string())];
    let res = post_signed(server.port, "github", BODY, headers).await;
    assert_eq!(res.status(), 400);

    server.handle.abort();
}

#[tokio::test]
async fn stripe_style_webhook_checks_the_timestamp() {
    let server = serve().await;

    let headers = sign(&server.stripe, BODY.as_bytes(), now());
    let res = post_signed(server.port, "stripe", BODY, headers).await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), BODY.len().to_string());

    let stale = sign(&server.stripe, BODY.as_bytes(), now() - 3600);
    let res = post_signed(server.port, "stripe", BODY, stale).await;
    assert_eq!(res.status(), 401);

    server.handle.abort();
}

#[tokio::test]
async fn unconfigured_source_is_a_server_error() {
    let server = serve().await;

    let headers = sign(&server.github, BODY.as_bytes(), now());
    let res = post_signed(server.port, "missing", BODY, headers).await;
    assert_eq!(res.status(), 500);

    server.handle.abort();
}
