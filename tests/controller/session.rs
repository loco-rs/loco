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
        session = session.remember()?;
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

/// Sets the user without `renew()`: the id must rotate by itself.
async fn promote(Query(p): Query<LoginParams>, session: Session) -> Result<Response> {
    Ok((session.set_user_id(p.user), format::text("promoted")?).into_response())
}

/// Ends the session, then writes a flash message to the fresh one.
async fn logout_flash(session: Session) -> Result<Response> {
    let session = session.clear().insert("flash", &"bye")?;
    Ok((session, format::text("bye")?).into_response())
}

/// A request that loads the session, takes a while, then writes it back.
async fn slow(session: Session) -> Result<Response> {
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    Ok((session.insert("late", &1)?, format::text("slow")?).into_response())
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
                .add("/promote", get(promote))
                .add("/logout_flash", get(logout_flash))
                .add("/slow", get(slow))
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
    let err = session_store::validate(&ctx.config)
        .unwrap_err()
        .to_string();
    assert!(err.contains("remember"), "{err}");
}

// ---- review findings -------------------------------------------------------

#[tokio::test]
async fn signing_in_as_someone_else_drops_the_previous_users_remember_cookie() {
    let App { mut server, ctx } = app(|_| {}).await;
    let alice = server
        .get("/login?user=alice&remember=true")
        .await
        .cookie("loco_remember");

    // Same browser, session still live: bob signs in (without remember).
    let res = server.get("/login?user=bob").await;
    let cleared = res.cookie("loco_remember");
    assert_eq!(cleared.value(), "", "the remember cookie is removed");
    assert_eq!(
        scalar(&ctx, "SELECT count(*) FROM loco_remember_tokens").await,
        0
    );

    // Alice's cookie, replayed once bob's session is gone, restores nobody.
    server.clear_cookies();
    server.add_cookie(alice);
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn a_flash_after_clear_still_revokes_the_remember_series() {
    let App { mut server, ctx } = app(|_| {}).await;
    let remember = server
        .get("/login?user=alice&remember=true")
        .await
        .cookie("loco_remember");
    server.get("/logout_flash").await.assert_text("bye");
    assert_eq!(
        scalar(&ctx, "SELECT count(*) FROM loco_remember_tokens").await,
        0
    );
    server.clear_cookies();
    server.add_cookie(remember);
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn set_user_id_without_renew_still_rotates_the_session_id() {
    let App { mut server, .. } = app(|_| {}).await;
    server.get("/count").await;
    let planted = server.get("/count").await.cookie("loco_session");

    server
        .get("/promote?user=alice")
        .await
        .assert_text("promoted");
    server.get("/me").await.assert_text("alice");

    // Whoever planted the old id is not signed in as alice.
    server.clear_cookies();
    server.add_cookie(planted);
    server.get("/me").await.assert_status_unauthorized();
}

#[tokio::test]
async fn a_slow_request_cannot_resurrect_a_signed_out_session() {
    let App { server, ctx } = app(|_| {}).await;
    let cookie = server.get("/login?user=alice").await.cookie("loco_session");

    let slow = server.get("/slow").add_cookie(cookie.clone());
    let out = async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        server.get("/logout").add_cookie(cookie.clone()).await
    };
    let (_, _) = tokio::join!(slow, out);

    assert_eq!(scalar(&ctx, "SELECT count(*) FROM loco_sessions").await, 0);
    server
        .get("/me")
        .add_cookie(cookie)
        .await
        .assert_status_unauthorized();
}

#[tokio::test]
async fn parallel_remember_restores_hand_out_one_working_token() {
    let App { mut server, ctx } = app(|_| {}).await;
    let t0 = server
        .get("/login?user=alice&remember=true")
        .await
        .cookie("loco_remember");
    server.clear_cookies();

    let (a, b, c) = tokio::join!(
        server.get("/me").add_cookie(t0.clone()),
        server.get("/me").add_cookie(t0.clone()),
        server.get("/me").add_cookie(t0.clone()),
    );
    let mut issued = vec![];
    for res in [a, b, c] {
        res.assert_text("alice");
        issued.push(res.cookie("loco_remember"));
    }
    // Whichever response the browser keeps must still work: no false theft.
    for cookie in issued {
        server.clear_cookies();
        server.add_cookie(cookie);
        server.get("/me").await.assert_text("alice");
    }
    assert_eq!(
        scalar(&ctx, "SELECT count(*) FROM loco_remember_tokens").await,
        1
    );
}

#[tokio::test]
async fn a_lost_rotation_response_is_reissued_not_flagged_as_theft() {
    let App { mut server, .. } = app(|_| {}).await;
    let t0 = server
        .get("/login?user=alice&remember=true")
        .await
        .cookie("loco_remember");

    // The rotation response never reaches the browser (we drop it) ...
    server.clear_cookies();
    server.add_cookie(t0.clone());
    server.get("/me").await.assert_text("alice");

    // ... so it presents the old token again, and is handed the current one.
    server.clear_cookies();
    server.add_cookie(t0);
    let res = server.get("/me").await;
    res.assert_text("alice");
    let reissued = res.cookie("loco_remember");
    server.clear_cookies();
    server.add_cookie(reissued);
    server.get("/me").await.assert_text("alice");
}

#[tokio::test]
async fn the_session_cookie_ends_with_the_browser_so_remember_me_can_restore() {
    let App { server, .. } = app(|_| {}).await;
    let res = server.get("/login?user=alice").await;
    assert!(
        res.cookie("loco_session").max_age().is_none(),
        "a server-side session cookie must not outlive the remember-me cookie"
    );
}

#[tokio::test]
async fn reading_a_session_late_in_its_life_slides_the_idle_timeout() {
    let App { server, ctx } = app(|s| s.ttl = Some(1000)).await;
    server.get("/login?user=alice").await;
    scalar(&ctx, "SELECT count(*) FROM loco_sessions").await;
    ctx.db
        .execute_raw(Statement::from_string(
            ctx.db.get_database_backend(),
            format!(
                "UPDATE loco_sessions SET expires_at = {}",
                chrono::Utc::now().timestamp() + 100
            ),
        ))
        .await
        .unwrap();

    // A request that only reads still extends it.
    server.get("/maybe").await.assert_text("alice");
    let left =
        scalar(&ctx, "SELECT expires_at FROM loco_sessions").await - chrono::Utc::now().timestamp();
    assert!(left > 900, "expected about 1000s left, got {left}");
}

#[tokio::test]
async fn theft_signs_the_user_out_of_every_device() {
    let App { mut server, ctx } = app(|_| {}).await;
    let laptop = server.get("/login?user=alice").await.cookie("loco_session");
    server.clear_cookies();
    let stolen = server
        .get("/login?user=alice&remember=true")
        .await
        .cookie("loco_remember");
    server.clear_cookies();
    server.add_cookie(stolen.clone());
    // The thief's first use works and mints a session of its own ...
    let res = server.get("/me").await;
    res.assert_text("alice");
    let minted = res.cookie("loco_session");

    // ... until the owner's rotation makes the copy stale and it is replayed.
    ctx.db
        .execute_unprepared("UPDATE loco_remember_tokens SET rotated_at = rotated_at - 600")
        .await
        .unwrap();
    server.clear_cookies();
    server.add_cookie(stolen);
    server.get("/me").await.assert_status_unauthorized();

    for cookie in [laptop, minted] {
        server.clear_cookies();
        server.add_cookie(cookie);
        server.get("/me").await.assert_status_unauthorized();
    }
    assert_eq!(scalar(&ctx, "SELECT count(*) FROM loco_sessions").await, 0);
}

#[tokio::test]
async fn the_remember_cookie_does_not_slide_when_it_rotates() {
    let App { mut server, ctx } = app(|s| s.remember_for = Some(100_000)).await;
    let first = server
        .get("/login?user=alice&remember=true")
        .await
        .cookie("loco_remember");
    let fixed = scalar(&ctx, "SELECT expires_at FROM loco_remember_tokens").await;

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    server.clear_cookies();
    server.add_cookie(first.clone());
    let res = server.get("/me").await;
    res.assert_text("alice");
    let rotated = res.cookie("loco_remember");

    assert_eq!(
        scalar(&ctx, "SELECT expires_at FROM loco_remember_tokens").await,
        fixed,
        "the series keeps the lifetime it was given at sign-in"
    );
    assert!(
        rotated.max_age().unwrap() < first.max_age().unwrap(),
        "the cookie counts down with it"
    );
}

// ---- the redis store through a router --------------------------------------

#[cfg(feature = "cache_redis")]
mod on_redis {
    use bb8_redis::redis;
    use testcontainers::{
        core::{ContainerPort, WaitFor},
        runners::AsyncRunner,
        ContainerAsync, GenericImage,
    };

    use super::*;

    async fn container() -> (String, ContainerAsync<GenericImage>) {
        let container = GenericImage::new("redis", "7-alpine")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("start redis");
        let port = container
            .get_host_port_ipv4(6379)
            .await
            .expect("redis port");
        (format!("redis://127.0.0.1:{port}"), container)
    }

    async fn redis_app(url: &str) -> App {
        let url = url.to_string();
        app(move |s| {
            s.store = SessionStoreKind::Redis;
            s.redis_uri = Some(url);
        })
        .await
    }

    async fn count_keys(url: &str, pattern: &str) -> usize {
        let client = redis::Client::open(url).unwrap();
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .unwrap();
        keys.len()
    }

    #[tokio::test]
    async fn sign_in_rotates_and_sign_out_deletes() {
        let (url, _container) = container().await;
        let App { server, .. } = redis_app(&url).await;

        server.get("/count").await.assert_text("1");
        let before = server.get("/count").await.cookie("loco_session");
        assert_eq!(count_keys(&url, "loco:session:s:*").await, 1);

        let after = server.get("/login?user=alice").await.cookie("loco_session");
        assert_ne!(before.value(), after.value(), "sign-in mints a new id");
        assert_eq!(count_keys(&url, "loco:session:s:*").await, 1, "old id gone");
        server.get("/me").await.assert_text("alice");

        server.get("/logout").await.assert_text("signed out");
        assert_eq!(count_keys(&url, "loco:session:s:*").await, 0);
        assert_eq!(count_keys(&url, "loco:session:u:*").await, 0);
        server.get("/me").await.assert_status_unauthorized();
    }

    #[tokio::test]
    async fn revoking_a_user_ends_every_device() {
        let (url, _container) = container().await;
        let App { mut server, .. } = redis_app(&url).await;

        let phone = server.get("/login?user=alice").await.cookie("loco_session");
        server.clear_cookies();
        let laptop = server.get("/login?user=alice").await.cookie("loco_session");
        server.clear_cookies();
        server.get("/login?user=bob").await;
        assert_eq!(count_keys(&url, "loco:session:s:*").await, 3);

        server.get("/revoke?user=alice").await.assert_text("2");
        assert_eq!(count_keys(&url, "loco:session:s:*").await, 1, "bob stays");

        for cookie in [phone, laptop] {
            server.clear_cookies();
            server.add_cookie(cookie);
            server.get("/me").await.assert_status_unauthorized();
        }
    }

    #[tokio::test]
    async fn revoking_a_user_reaches_a_series_that_has_rotated() {
        let (url, _container) = container().await;
        let App { mut server, .. } = redis_app(&url).await;

        let t0 = server
            .get("/login?user=alice&remember=true")
            .await
            .cookie("loco_remember");
        let mut latest = t0;
        for _ in 0..3 {
            server.clear_cookies();
            server.add_cookie(latest.clone());
            latest = server.get("/me").await.cookie("loco_remember");
        }
        server.get("/revoke?user=alice").await;
        assert_eq!(count_keys(&url, "loco:session:*").await, 0);

        server.clear_cookies();
        server.add_cookie(latest);
        server.get("/me").await.assert_status_unauthorized();
    }

    #[tokio::test]
    async fn remember_me_restores_a_lost_session() {
        let (url, _container) = container().await;
        let App { mut server, .. } = redis_app(&url).await;

        let remember = server
            .get("/login?user=alice&remember=true")
            .await
            .cookie("loco_remember");
        assert_eq!(count_keys(&url, "loco:session:r:*").await, 1);

        // The browser was closed: the session cookie is gone, remember stays.
        server.clear_cookies();
        server.add_cookie(remember.clone());
        let res = server.get("/me").await;
        res.assert_text("alice");
        let rotated = res.cookie("loco_remember");
        assert_ne!(rotated.value(), remember.value(), "the token rotates");
        assert_eq!(count_keys(&url, "loco:session:r:*").await, 1);

        // Signing out takes the series with it.
        server.get("/logout").await.assert_text("signed out");
        assert_eq!(count_keys(&url, "loco:session:r:*").await, 0);
        server.clear_cookies();
        server.add_cookie(rotated);
        server.get("/me").await.assert_status_unauthorized();
    }
}
