//! Server-side sessions (`auth.session.store: database`) through a router:
//! sign-in rotates the id, sign-out and revoke-all delete server records, and
//! remember-me restores, rotates and detects theft. The stores themselves are
//! covered per backend (sqlite, Postgres, Redis) in `session_store/tests.rs`.

use axum::extract::Query;
use loco_rs::{
    auth::{
        session::{CurrentUser, MaybeCurrentUser, Session},
        session_store,
    },
    config::{Auth, SessionConfig, SessionStoreKind},
    controller::AppRoutes,
    prelude::*,
    tests_cfg::db::AppHook,
    TestServer,
};
use sea_orm::{ConnectionTrait, Statement};
use serde::Deserialize;

#[derive(Deserialize)]
struct LoginParams {
    user: String,
    #[serde(default)]
    remember: bool,
}

async fn login(Query(p): Query<LoginParams>, session: Session) -> Result<Response> {
    let mut session = session.renew().set_user_id(p.user);
    if p.remember {
        session = session.remember();
    }
    Ok((session, format::text("signed in")?).into_response())
}

async fn logout(session: Session) -> Result<Response> {
    Ok((session.clear(), format::text("signed out")?).into_response())
}

async fn me(CurrentUser(id): CurrentUser<String>) -> Result<Response> {
    format::text(&id)
}

async fn maybe(MaybeCurrentUser(id): MaybeCurrentUser<String>) -> Result<Response> {
    format::text(id.as_deref().unwrap_or("anonymous"))
}

async fn count(session: Session) -> Result<Response> {
    let n: u32 = session.get("n").unwrap_or(0) + 1;
    Ok((session.insert("n", &n)?, format::text(&n.to_string())?).into_response())
}

async fn revoke_all(
    State(ctx): State<AppContext>,
    Query(p): Query<LoginParams>,
) -> Result<Response> {
    let n = session_store::sign_out_everywhere(&ctx, &p.user).await?;
    format::text(&n.to_string())
}

struct App {
    server: TestServer,
    ctx: AppContext,
}

async fn app(configure: impl FnOnce(&mut SessionConfig)) -> App {
    let mut ctx = loco_rs::tests_cfg::app::get_app_context().await;
    let mut session = SessionConfig {
        secret: Some("an-integration-test-secret-that-is-long-enough".to_string()),
        store: SessionStoreKind::Database,
        ..Default::default()
    };
    configure(&mut session);
    ctx.config.auth = Some(Auth {
        session: Some(session),
        ..Default::default()
    });
    let router = AppRoutes::empty()
        .add_route(
            Routes::new()
                .add("/login", get(login))
                .add("/logout", get(logout))
                .add("/me", get(me))
                .add("/maybe", get(maybe))
                .add("/count", get(count))
                .add("/revoke", get(revoke_all)),
        )
        .to_router::<AppHook>(ctx.clone(), axum::Router::new())
        .unwrap();
    let server = TestServer::builder().save_cookies().build(router).unwrap();
    App { server, ctx }
}

async fn scalar(ctx: &AppContext, sql: &str) -> i64 {
    let row = ctx
        .db
        .query_one_raw(Statement::from_string(
            ctx.db.get_database_backend(),
            sql.to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get_by_index(0).unwrap()
}

#[tokio::test]
async fn the_cookie_holds_an_opaque_id_and_the_data_lives_on_the_server() {
    let App { server, ctx } = app(|_| {}).await;
    server.get("/maybe").await.assert_text("anonymous");

    let res = server.get("/login?user=alice").await;
    res.assert_text("signed in");
    let cookie = res.cookie("loco_session");
    assert_eq!(cookie.http_only(), Some(true));
    assert!(
        !cookie.value().contains("alice"),
        "no session data in the cookie"
    );
    assert_eq!(scalar(&ctx, "SELECT count(*) FROM loco_sessions").await, 1);

    server.get("/me").await.assert_text("alice");
    server.get("/count").await.assert_text("1");
    server.get("/count").await.assert_text("2");
    // still one row: updates are in place
    assert_eq!(scalar(&ctx, "SELECT count(*) FROM loco_sessions").await, 1);
}

#[tokio::test]
async fn signing_in_rotates_the_session_id() {
    let App { mut server, ctx } = app(|_| {}).await;
    // An anonymous session already exists (the attacker's planted one).
    server.get("/count").await;
    let planted = server.get("/count").await.cookie("loco_session");

    server
        .get("/login?user=alice")
        .await
        .assert_text("signed in");
    let after = server.get("/me").await;
    after.assert_text("alice");

    // The pre-login id is dead.
    server.clear_cookies();
    server.add_cookie(planted);
    server.get("/me").await.assert_status_unauthorized();
    server.get("/count").await.assert_text("1");
    assert_eq!(
        scalar(
            &ctx,
            "SELECT count(*) FROM loco_sessions WHERE user_key = 'alice'"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn signing_out_deletes_the_server_record() {
    let App { mut server, ctx } = app(|_| {}).await;
    let cookie = server.get("/login?user=alice").await.cookie("loco_session");
    server.get("/logout").await;
    assert_eq!(scalar(&ctx, "SELECT count(*) FROM loco_sessions").await, 0);

    // Replaying the cookie of a signed-out session gets nothing.
    server.clear_cookies();
    server.add_cookie(cookie);
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn expired_sessions_are_absent() {
    let App { server, ctx } = app(|_| {}).await;
    server.get("/login?user=alice").await;
    server.get("/me").await.assert_text("alice");
    ctx.db
        .execute_unprepared("UPDATE loco_sessions SET expires_at = 1")
        .await
        .unwrap();
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn revoking_a_user_ends_every_device() {
    let App { mut server, ctx } = app(|_| {}).await;
    let phone = server.get("/login?user=alice").await.cookie("loco_session");
    server.clear_cookies();
    server.get("/login?user=alice").await;
    server.clear_cookies();
    server.get("/login?user=bob").await;

    server.get("/revoke?user=alice").await.assert_text("2");

    server.add_cookie(phone);
    server.get("/me").await.assert_status_unauthorized();
    assert_eq!(scalar(&ctx, "SELECT count(*) FROM loco_sessions").await, 1);
}

#[tokio::test]
async fn remember_me_restores_a_lost_session_and_rotates_the_token() {
    let App { mut server, .. } = app(|_| {}).await;
    let res = server.get("/login?user=alice&remember=true").await;
    let remember = res.cookie("loco_remember");
    assert_eq!(remember.http_only(), Some(true));
    assert!(remember.max_age().is_some(), "long lived");

    // The browser was closed: only the remember cookie survives.
    server.clear_cookies();
    server.add_cookie(remember.clone());
    let res = server.get("/me").await;
    res.assert_text("alice");
    let rotated = res.cookie("loco_remember");
    assert_ne!(rotated.value(), remember.value(), "token rotates");
    assert!(res.maybe_cookie("loco_session").is_some(), "new session");

    // And it keeps working with the rotated cookie.
    server.clear_cookies();
    server.add_cookie(rotated);
    server.get("/me").await.assert_text("alice");
}

#[tokio::test]
async fn without_opting_in_there_is_no_remember_cookie() {
    let App { server, .. } = app(|_| {}).await;
    let res = server.get("/login?user=alice").await;
    assert!(res.maybe_cookie("loco_remember").is_none());
}

#[tokio::test]
async fn a_replayed_remember_token_is_theft_and_revokes_the_series() {
    let App { mut server, ctx } = app(|_| {}).await;
    let stolen = server
        .get("/login?user=alice&remember=true")
        .await
        .cookie("loco_remember");

    // The owner uses it: the token rotates.
    server.clear_cookies();
    server.add_cookie(stolen.clone());
    let owner = server.get("/me").await.cookie("loco_remember");

    // Past the parallel-request grace window the thief replays the old one.
    ctx.db
        .execute_unprepared("UPDATE loco_remember_tokens SET rotated_at = rotated_at - 600")
        .await
        .unwrap();
    server.clear_cookies();
    server.add_cookie(stolen);
    server.get("/me").await.assert_status_unauthorized();
    assert_eq!(
        scalar(&ctx, "SELECT count(*) FROM loco_remember_tokens").await,
        0,
        "the whole series is gone"
    );

    // So the owner's newer token no longer works either.
    server.clear_cookies();
    server.add_cookie(owner);
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn revoking_a_user_kills_remember_tokens_too() {
    let App { mut server, .. } = app(|_| {}).await;
    let remember = server
        .get("/login?user=alice&remember=true")
        .await
        .cookie("loco_remember");
    server.get("/revoke?user=alice").await;

    server.clear_cookies();
    server.add_cookie(remember);
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn remember_me_with_the_cookie_store_fails_at_boot() {
    let mut ctx = loco_rs::tests_cfg::app::get_app_context().await;
    ctx.config.auth = Some(Auth {
        session: Some(SessionConfig {
            secret: Some("an-integration-test-secret-that-is-long-enough".to_string()),
            remember_for: Some(3600),
            ..Default::default()
        }),
        ..Default::default()
    });
    let err = session_store::init(&ctx).await.unwrap_err().to_string();
    assert!(err.contains("remember"), "{err}");
}
