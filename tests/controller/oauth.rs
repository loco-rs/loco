//! End-to-end OAuth sign-in against an in-process fake provider.
//!
//! The fake runs on a random port and behaves like GitHub: `/authorize`
//! issues a code bound to the PKCE challenge and redirect URI, `/token`
//! insists on both, and `/user` + `/user/emails` answer only to the token.
//! The test plays the browser between the Loco app (an `axum-test` server
//! that keeps cookies) and the fake.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Form, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use loco_rs::{
    auth::oauth::{self, Identity, OAuthHandler, SignIn},
    config::{Auth, OAuthConfig, OAuthProviderConfig, SessionConfig},
    controller::AppRoutes,
    prelude::*,
    tests_cfg::db::AppHook,
    TestServer,
};
use serde_json::json;
use sha2::{Digest, Sha256};

const CLIENT_ID: &str = "fake-client";
const CLIENT_SECRET: &str = "fake-secret";
const TOKEN: &str = "gho_fake_access_token";

#[derive(Default)]
struct Fake {
    /// code -> (code_challenge, redirect_uri)
    codes: HashMap<String, (Option<String>, String)>,
    /// Form fields of every token request.
    token_requests: Vec<HashMap<String, String>>,
    login: String,
}

type Shared = Arc<Mutex<Fake>>;

async fn authorize(
    State(fake): State<Shared>,
    Query(q): Query<HashMap<String, String>>,
) -> Redirect {
    let code = format!("code-{}", fake.lock().unwrap().codes.len());
    fake.lock().unwrap().codes.insert(
        code.clone(),
        (q.get("code_challenge").cloned(), q["redirect_uri"].clone()),
    );
    Redirect::to(&format!(
        "{}?code={code}&state={}",
        q["redirect_uri"], q["state"]
    ))
}

async fn token(
    State(fake): State<Shared>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut fake = fake.lock().unwrap();
    fake.token_requests.push(form.clone());
    let error = |code: &str| Json(json!({ "error": code, "error_description": "nope" }));
    if form.get("client_id").map(String::as_str) != Some(CLIENT_ID)
        || form.get("client_secret").map(String::as_str) != Some(CLIENT_SECRET)
    {
        return error("invalid_client");
    }
    let Some((challenge, redirect_uri)) = form.get("code").and_then(|c| fake.codes.remove(c))
    else {
        // GitHub-style: a bad code is an error body with a 200.
        return error("bad_verification_code");
    };
    if form.get("redirect_uri") != Some(&redirect_uri) {
        return error("redirect_uri_mismatch");
    }
    let verified = form
        .get("code_verifier")
        .map(|v| URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())));
    if verified != challenge {
        return error("invalid_grant");
    }
    Json(json!({ "access_token": TOKEN, "token_type": "bearer", "scope": "read:user,user:email" }))
}

fn authorized(headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        == Some(&format!("Bearer {TOKEN}"))
}

async fn user(State(fake): State<Shared>, headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let login = fake.lock().unwrap().login.clone();
    Json(
        json!({ "id": 583_231, "login": login, "name": "The Octocat", "email": null,
                 "avatar_url": "https://avatars.example/u/583231" }),
    )
    .into_response()
}

async fn emails(headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    Json(json!([
        { "email": "secondary@example.com", "primary": false, "verified": true },
        { "email": "octocat@example.com", "primary": true, "verified": true }
    ]))
    .into_response()
}

async fn start_fake() -> (String, Shared) {
    let fake: Shared = Arc::new(Mutex::new(Fake {
        login: "octocat".to_string(),
        ..Default::default()
    }));
    let app = Router::new()
        .route("/login/oauth/authorize", get(authorize))
        .route("/login/oauth/access_token", post(token))
        .route("/api/v3/user", get(user))
        .route("/api/v3/user/emails", get(emails))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, fake)
}

struct Handler;

#[async_trait]
impl OAuthHandler for Handler {
    async fn on_sign_in(_ctx: &AppContext, identity: &Identity) -> Result<SignIn> {
        if identity.login.as_deref() == Some("blocked") {
            return Err(Error::Unauthorized("not on the allowlist".to_string()));
        }
        // The session id carries what the test wants to assert on.
        Ok(SignIn::new(format!(
            "{}:{}:{}",
            identity.provider,
            identity.uid,
            identity.email.as_deref().unwrap_or("-")
        ))
        .redirect_to("/welcome"))
    }
}

async fn me(CurrentUser(id): CurrentUser<String>) -> Result<Response> {
    format::text(&id)
}

async fn maybe(MaybeCurrentUser(id): MaybeCurrentUser<String>) -> Result<Response> {
    format::text(id.as_deref().unwrap_or("anonymous"))
}

/// An app route next to the mounted ones, like a GitHub App's install
/// callback.
async fn setup() -> Result<Response> {
    format::text("app setup route")
}

struct App {
    server: TestServer,
    fake: Shared,
}

async fn app(configure: impl FnOnce(&mut OAuthConfig)) -> App {
    let (fake_url, fake) = start_fake().await;
    let mut ctx = loco_rs::tests_cfg::app::get_app_context().await;
    let mut oauth = OAuthConfig {
        base_url: Some("http://app.test".to_string()),
        ..Default::default()
    };
    oauth.providers.insert(
        "github".to_string(),
        OAuthProviderConfig {
            client_id: CLIENT_ID.to_string(),
            client_secret: CLIENT_SECRET.to_string(),
            // GitHub Enterprise shape: one base URL, API under /api/v3.
            base_url: Some(fake_url),
            ..Default::default()
        },
    );
    configure(&mut oauth);
    ctx.config.auth = Some(Auth {
        session: Some(SessionConfig {
            secret: Some("an-integration-test-secret-that-is-long-enough".to_string()),
            sign_in_path: Some("/login".to_string()),
            ..Default::default()
        }),
        oauth: Some(oauth),
        ..Default::default()
    });

    let router = AppRoutes::empty()
        .add_route(oauth::routes::<Handler>().add("/github/setup", get(setup)))
        .add_route(Routes::new().add("/me", get(me)).add("/maybe", get(maybe)))
        .to_router::<AppHook>(ctx, axum::Router::new())
        .unwrap();
    let server = TestServer::builder().save_cookies().build(router);
    App { server, fake }
}

/// The `Location` of a response.
macro_rules! location {
    ($res:expr) => {
        $res.header(header::LOCATION).to_str().unwrap().to_string()
    };
}

/// Follows the provider's authorize redirect the way a browser would, and
/// returns the app-relative callback path it lands on.
async fn visit_provider(authorize_url: &str) -> String {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let res = client.get(authorize_url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let callback = res.headers()[header::LOCATION].to_str().unwrap();
    callback
        .strip_prefix("http://app.test")
        .expect("the provider redirects to the configured base_url")
        .to_string()
}

#[tokio::test]
async fn signs_in_end_to_end_and_out_again() {
    let App { server, fake } = app(|_| {}).await;

    // Signed out: browsers are sent to sign in with a return_to.
    let res = server
        .get("/me?tab=1")
        .add_header(header::ACCEPT, "text/html")
        .await;
    res.assert_status(StatusCode::SEE_OTHER);
    assert_eq!(location!(res), "/login?return_to=%2Fme%3Ftab%3D1");

    let res = server.get("/auth/github?return_to=/me").await;
    res.assert_status(StatusCode::SEE_OTHER);
    let authorize = location!(res);
    let set_cookie = res.header(header::SET_COOKIE).to_str().unwrap().to_string();
    assert!(
        set_cookie.starts_with("loco_oauth_state_github="),
        "{set_cookie}"
    );
    assert!(set_cookie.contains("HttpOnly") && set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Max-Age=600"), "{set_cookie}");

    let url = url::Url::parse(&authorize).unwrap();
    let q: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(q["client_id"], CLIENT_ID);
    assert_eq!(q["redirect_uri"], "http://app.test/auth/github/callback");
    assert_eq!(q["scope"], "read:user user:email");
    assert_eq!(q["code_challenge_method"], "S256");

    let callback = visit_provider(&authorize).await;
    let res = server.get(&callback).await;
    res.assert_status(StatusCode::SEE_OTHER);
    assert_eq!(
        location!(res),
        "/me",
        "return_to wins over SignIn::redirect_to"
    );
    let cookies: Vec<_> = res
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();
    assert!(
        cookies
            .iter()
            .any(|c| c.starts_with("loco_oauth_state_github=;") && c.contains("Max-Age=0")),
        "the state cookie is cleared: {cookies:?}"
    );
    assert!(
        cookies.iter().any(|c| c.starts_with("loco_session=")),
        "{cookies:?}"
    );

    let token_request = fake.lock().unwrap().token_requests[0].clone();
    assert_eq!(token_request["grant_type"], "authorization_code");
    assert!(token_request.contains_key("code_verifier"));

    server
        .get("/me")
        .await
        .assert_text("github:583231:octocat@example.com");
    server
        .get("/maybe")
        .await
        .assert_text("github:583231:octocat@example.com");

    // Replaying the callback fails: the state was single-use.
    let replay = server.get(&callback).await;
    replay.assert_status(StatusCode::UNAUTHORIZED);
    assert!(replay.text().contains("expired"));

    let res = server.get("/auth/sign_out").await;
    res.assert_status(StatusCode::SEE_OTHER);
    assert_eq!(location!(res), "/");
    server.get("/me").await.assert_status_unauthorized();
    server.get("/maybe").await.assert_text("anonymous");
}

#[tokio::test]
async fn without_return_to_the_handler_picks_the_landing_page() {
    let App { server, .. } = app(|_| {}).await;
    let authorize = location!(server.get("/auth/github").await);
    let res = server.get(&visit_provider(&authorize).await).await;
    assert_eq!(location!(res), "/welcome");
}

#[tokio::test]
async fn an_off_site_return_to_is_ignored() {
    let App { server, .. } = app(|_| {}).await;
    for evil in [
        "//evil.example/x",
        "https://evil.example",
        "/\\evil.example",
    ] {
        let start = format!("/auth/github?return_to={}", urlencode(evil));
        let authorize = location!(server.get(&start).await);
        let res = server.get(&visit_provider(&authorize).await).await;
        assert_eq!(location!(res), "/welcome", "{evil}");
    }
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[tokio::test]
async fn a_forged_state_is_rejected_without_calling_the_provider() {
    let App { server, fake } = app(|_| {}).await;
    let authorize = location!(server.get("/auth/github").await);
    let callback = visit_provider(&authorize).await;
    let url = url::Url::parse(&format!("http://app.test{callback}")).unwrap();
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .unwrap()
        .1
        .into_owned();

    let res = server
        .get(&format!("/auth/github/callback?code={code}&state=forged"))
        .await;
    res.assert_status(StatusCode::UNAUTHORIZED);
    assert!(fake.lock().unwrap().token_requests.is_empty());
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn a_callback_without_a_state_cookie_is_rejected() {
    let App { server, fake } = app(|_| {}).await;
    let res = server
        .get("/auth/github/callback?code=code-0&state=anything")
        .await;
    res.assert_status(StatusCode::UNAUTHORIZED);
    assert!(res.text().contains("expired"));
    assert!(res.text().contains("href=\"/auth/github\""), "retry link");
    assert!(fake.lock().unwrap().token_requests.is_empty());
}

#[tokio::test]
async fn failures_redirect_to_failure_path_when_configured() {
    let App { server, .. } = app(|o| o.failure_path = Some("/login".to_string())).await;
    server.get("/auth/github").await;
    let res = server
        .get("/auth/github/callback?error=access_denied&error_description=%3Cscript%3E")
        .await;
    res.assert_status(StatusCode::SEE_OTHER);
    assert_eq!(location!(res), "/login?error=cancelled&provider=github");
}

#[tokio::test]
async fn a_refused_code_is_a_token_exchange_failure() {
    let App { server, .. } = app(|o| o.failure_path = Some("/login".to_string())).await;
    let authorize = location!(server.get("/auth/github").await);
    let callback = visit_provider(&authorize).await;
    let tampered = callback.replace("code=code-0", "code=wrong");
    let res = server.get(&tampered).await;
    assert_eq!(
        location!(res),
        "/login?error=token_exchange&provider=github"
    );
}

#[tokio::test]
async fn on_sign_in_can_deny() {
    let App { server, fake } = app(|o| o.failure_path = Some("/login".to_string())).await;
    fake.lock().unwrap().login = "blocked".to_string();
    let authorize = location!(server.get("/auth/github").await);
    let res = server.get(&visit_provider(&authorize).await).await;
    assert_eq!(location!(res), "/login?error=denied&provider=github");
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn unknown_providers_are_not_found_and_app_routes_coexist() {
    let App { server, .. } = app(|_| {}).await;
    server.get("/auth/gitlab").await.assert_status_not_found();
    server
        .get("/auth/github/setup")
        .await
        .assert_text("app setup route");
}

#[tokio::test]
async fn pkce_can_be_turned_off() {
    let App { server, fake } = app(|o| {
        o.providers.get_mut("github").unwrap().pkce = Some(false);
    })
    .await;
    let authorize = location!(server.get("/auth/github").await);
    assert!(!authorize.contains("code_challenge"));
    let res = server.get(&visit_provider(&authorize).await).await;
    assert_eq!(location!(res), "/welcome");
    assert!(!fake.lock().unwrap().token_requests[0].contains_key("code_verifier"));
}
