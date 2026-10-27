use serde_json::json;

use super::*;
use crate::config::{Auth, SessionConfig};

fn store(backend: Arc<dyn Backend>, kind: SessionStoreKind) -> SessionStore {
    SessionStore {
        backend,
        kind,
        ttl: 3600,
        remember_for: 7200,
        secret: b"a-fixed-test-secret-for-remember-tokens".to_vec(),
    }
}

fn data(user: &str) -> BTreeMap<String, Value> {
    BTreeMap::from([
        (super::super::session::USER_ID_KEY.to_string(), json!(user)),
        ("cart".to_string(), json!(3)),
    ])
}

async fn insert(store: &SessionStore, id: &str, record: Record) {
    store
        .apply(vec![Op::Insert {
            id: id.to_string(),
            record,
        }])
        .await
        .unwrap();
}

async fn update(store: &SessionStore, id: &str, record: Record) {
    store
        .apply(vec![Op::Update {
            id: id.to_string(),
            record,
        }])
        .await
        .unwrap();
}

async fn put(store: &SessionStore, id: &str, user: &str) {
    insert(store, id, store.record(data(user))).await;
}

/// The behaviour every backend must share.
async fn exercise(store: SessionStore) {
    // create / read
    put(&store, "sid-a", "alice").await;
    let rec = store.load("sid-a").await.unwrap().expect("stored");
    assert_eq!(rec.user.as_deref(), Some("alice"));
    assert_eq!(rec.data["cart"], json!(3));
    assert!(store.load("nope").await.unwrap().is_none());

    // update in place
    let mut again = store.record(data("alice"));
    again.data.insert("cart".to_string(), json!(4));
    update(&store, "sid-a", again).await;
    assert_eq!(
        store.load("sid-a").await.unwrap().unwrap().data["cart"],
        json!(4)
    );

    // expired rows read as absent
    let mut old = store.record(data("alice"));
    old.expires_at = now() - 5;
    insert(&store, "sid-old", old).await;
    assert!(store.load("sid-old").await.unwrap().is_none());
    store.purge_expired().await.unwrap();

    // an update never recreates a session that is gone
    update(&store, "sid-ghost", store.record(data("alice"))).await;
    assert!(store.load("sid-ghost").await.unwrap().is_none());

    // delete
    store
        .apply(vec![Op::Delete {
            id: "sid-a".to_string(),
        }])
        .await
        .unwrap();
    assert!(store.load("sid-a").await.unwrap().is_none());

    // revoke all: sessions and remember series, other users untouched
    put(&store, "sid-1", "bob").await;
    put(&store, "sid-2", "bob").await;
    put(&store, "sid-3", "carol").await;
    let (op, cookie) = store.new_remember("bob");
    store.apply(vec![op]).await.unwrap();
    let (series, token) = cookie.split_once('.').unwrap();
    store.revoke_user("bob").await.unwrap();
    assert!(store.load("sid-1").await.unwrap().is_none());
    assert!(store.load("sid-2").await.unwrap().is_none());
    assert!(store.load("sid-3").await.unwrap().is_some());
    assert!(matches!(
        store.restore(series, token).await.unwrap(),
        Restored::Rejected
    ));

    // remember: restore rotates, the previous token has a grace window, a
    // wrong token on a known series is theft and kills the series
    let (op, cookie) = store.new_remember("carol");
    store.apply(vec![op]).await.unwrap();
    let (series, first) = cookie.split_once('.').unwrap();
    let Restored::Ok { user, token, .. } = store.restore(series, first).await.unwrap() else {
        panic!("valid token rejected");
    };
    assert_eq!(user, "carol");
    let second = token.expect("token rotates");
    assert_ne!(second, first);
    // The replaced token is still good for a moment, and is answered with the
    // current one: a lost rotation response heals instead of looking like theft.
    let Restored::Ok { token: again, .. } = store.restore(series, first).await.unwrap() else {
        panic!("the replaced token is inside its grace window");
    };
    assert_eq!(again.as_deref(), Some(second.as_str()));
    assert!(matches!(
        store.restore(series, "guess").await.unwrap(),
        Restored::Rejected
    ));
    assert!(
        matches!(
            store.restore(series, &second).await.unwrap(),
            Restored::Rejected
        ),
        "the series is revoked after a mismatch"
    );
    assert!(matches!(
        store.restore("unknown", "x").await.unwrap(),
        Restored::Rejected
    ));
}

#[test]
fn tokens_are_long_and_distinct() {
    let (a, b) = (random_token(), random_token());
    assert_ne!(a, b);
    assert!(a.len() >= 43, "256 bits of base64url");
    assert_ne!(hash(&a), a);
}

#[test]
fn ct_eq_compares() {
    assert!(ct_eq("abc", "abc"));
    assert!(!ct_eq("abc", "abd"));
    assert!(!ct_eq("abc", "ab"));
}

#[test]
fn remember_needs_a_server_side_store() {
    let mut config = crate::tests_cfg::config::test_config();
    config.auth = Some(Auth {
        session: Some(SessionConfig {
            remember_for: Some(60),
            ..Default::default()
        }),
        ..Default::default()
    });
    let err = validate(&config).unwrap_err().to_string();
    assert!(err.contains("server-side session store"), "{err}");

    let session = config.auth.as_mut().unwrap().session.as_mut().unwrap();
    session.store = SessionStoreKind::Database;
    // remember-me that expires with the session never restores anything
    let err = validate(&config).unwrap_err().to_string();
    assert!(err.contains("longer than auth.session.ttl"), "{err}");

    let session = config.auth.as_mut().unwrap().session.as_mut().unwrap();
    session.ttl = Some(59);
    validate(&config).unwrap();
    let session = config.auth.as_mut().unwrap().session.as_mut().unwrap();
    session.ttl = Some(60);
    assert!(validate(&config).is_err());
}

#[test]
fn the_store_debug_output_has_no_key_material() {
    #[derive(Debug)]
    struct Nothing;
    #[async_trait]
    impl Backend for Nothing {
        async fn load(&self, _: &str) -> Result<Option<Record>> {
            Ok(None)
        }
        async fn insert(&self, _: &str, _: &Record) -> Result<()> {
            Ok(())
        }
        async fn insert_restored(&self, _: &str, _: &Record, _: &str) -> Result<bool> {
            Ok(false)
        }
        async fn update(&self, _: &str, _: &Record) -> Result<bool> {
            Ok(false)
        }
        async fn delete(&self, _: &str) -> Result<()> {
            Ok(())
        }
        async fn revoke_user(&self, _: &str) -> Result<u64> {
            Ok(0)
        }
        async fn remember_get(&self, _: &str) -> Result<Option<Remember>> {
            Ok(None)
        }
        async fn remember_insert(&self, _: &str, _: &Remember) -> Result<()> {
            Ok(())
        }
        async fn remember_cas(&self, _: &str, _: i64, _: &Remember) -> Result<bool> {
            Ok(false)
        }
        async fn remember_delete(&self, _: &str) -> Result<()> {
            Ok(())
        }
        async fn purge_expired(&self, _: i64) -> Result<u64> {
            Ok(0)
        }
    }
    let s = store(Arc::new(Nothing), SessionStoreKind::Database);
    assert!(!format!("{s:?}").contains("a-fixed-test-secret"));
}

/// Several requests carrying the same remember cookie arrive together (a page
/// with a handful of assets). One rotates; the rest must be answered with a
/// token that works, never with a theft verdict.
async fn parallel_restores_never_look_like_theft(store: SessionStore) {
    let (op, cookie) = store.new_remember("erin");
    store.apply(vec![op]).await.unwrap();
    let (series, first) = cookie.split_once('.').unwrap();

    let results =
        futures_util::future::join_all((0..8).map(|_| store.restore(series, first))).await;
    let mut issued = Vec::new();
    for result in results {
        match result.unwrap() {
            Restored::Ok { user, token, .. } => {
                assert_eq!(user, "erin");
                issued.push(token.expect("every answer carries a token"));
            }
            Restored::Rejected => panic!("a parallel restore was treated as theft"),
        }
    }
    // Whichever response the browser keeps, its token restores the session.
    for token in issued {
        assert!(
            matches!(
                store.restore(series, &token).await.unwrap(),
                Restored::Ok { .. }
            ),
            "a token handed out during a parallel restore must keep working"
        );
    }
}

#[cfg(feature = "with-db")]
#[tokio::test]
async fn parallel_restores_on_the_database_store() {
    let db = sqlite().await;
    let backend = db::Database::init(db).await.unwrap();
    parallel_restores_never_look_like_theft(store(Arc::new(backend), SessionStoreKind::Database))
        .await;
}

#[cfg(feature = "with-db")]
#[tokio::test]
async fn a_failed_write_rolls_the_whole_batch_back() {
    let db = sqlite().await;
    let store = store(
        Arc::new(db::Database::init(db).await.unwrap()),
        SessionStoreKind::Database,
    );
    put(&store, "keep", "frank").await;
    let result = store
        .apply(vec![
            Op::Delete {
                id: "keep".to_string(),
            },
            Op::Insert {
                id: "dup".to_string(),
                record: store.record(data("frank")),
            },
            Op::Insert {
                id: "dup".to_string(),
                record: store.record(data("frank")),
            },
        ])
        .await;
    assert!(result.is_err(), "the duplicate id must fail");
    assert!(
        store.load("keep").await.unwrap().is_some(),
        "the delete was rolled back with the rest"
    );
}

#[cfg(feature = "with-db")]
#[tokio::test]
async fn concurrent_first_use_on_postgres_does_not_race_on_ddl() {
    let (url, _container) = crate::tests_cfg::postgres::setup_postgres_database().await;
    let db = sea_orm::Database::connect(&url).await.unwrap();
    let results =
        futures_util::future::join_all((0..8).map(|_| db::Database::init(db.clone()))).await;
    for result in results {
        result.expect("concurrent init must be serialised, not fail on CREATE TABLE");
    }
}

#[cfg(feature = "with-db")]
async fn sqlite() -> sea_orm::DatabaseConnection {
    // One connection: every pooled connection to `:memory:` is its own database.
    let mut opt = sea_orm::ConnectOptions::new("sqlite::memory:");
    opt.max_connections(1).test_before_acquire(false);
    sea_orm::Database::connect(opt).await.unwrap()
}

#[cfg(feature = "with-db")]
#[tokio::test]
async fn database_store_on_sqlite() {
    let db = sqlite().await;
    let backend = db::Database::init(db.clone()).await.unwrap();
    // creating the tables is idempotent
    db::Database::init(db.clone()).await.unwrap();
    let store = store(Arc::new(backend), SessionStoreKind::Database);
    exercise(store.clone()).await;

    // only the hash of the id reaches the table
    put(&store, "plain-id", "dave").await;
    let rows =
        crate::tests_cfg::db::get_value(&db, "SELECT group_concat(id_hash) FROM loco_sessions")
            .await;
    assert!(!rows.contains("plain-id"), "{rows}");
    assert!(rows.contains(&hash("plain-id").to_lowercase()));
}

#[cfg(feature = "with-db")]
#[tokio::test]
async fn database_store_purges_expired_rows() {
    let db = sqlite().await;
    let store = store(
        Arc::new(db::Database::init(db.clone()).await.unwrap()),
        SessionStoreKind::Database,
    );
    let mut old = store.record(data("x"));
    old.expires_at = now() - 10;
    // Straight to the backend: `apply` purges one save in 64 by itself.
    store.backend.insert(&hash("gone"), &old).await.unwrap();
    assert_eq!(store.purge_expired().await.unwrap(), 1);
    let n = crate::tests_cfg::db::get_value(&db, "SELECT count(*) FROM loco_sessions").await;
    assert_eq!(n, "0");
}

#[cfg(feature = "with-db")]
#[tokio::test]
async fn database_store_on_postgres() {
    let (url, _container) = crate::tests_cfg::postgres::setup_postgres_database().await;
    let db = sea_orm::Database::connect(&url).await.unwrap();
    let backend = db::Database::init(db).await.unwrap();
    let store = store(Arc::new(backend), SessionStoreKind::Database);
    exercise(store.clone()).await;
    theft_ends_everything_the_user_has(store.clone()).await;
    rotation_keeps_the_series_expiry(store.clone()).await;
    a_restored_session_needs_its_series(store).await;
}

#[cfg(feature = "cache_redis")]
#[tokio::test]
async fn redis_store() {
    let (url, _container) = crate::tests_cfg::redis::setup_redis_container().await;
    let backend = redis::Redis::connect(&url).await.unwrap();
    exercise(store(Arc::new(backend), SessionStoreKind::Redis)).await;
}

#[cfg(feature = "cache_redis")]
#[tokio::test]
async fn parallel_restores_on_the_redis_store() {
    let (url, _container) = crate::tests_cfg::redis::setup_redis_container().await;
    let backend = redis::Redis::connect(&url).await.unwrap();
    parallel_restores_never_look_like_theft(store(Arc::new(backend), SessionStoreKind::Redis))
        .await;
}

#[cfg(feature = "cache_redis")]
#[tokio::test]
async fn redis_indexes_are_scored_by_expiry_and_hold_no_dead_members() {
    let (url, _container) = crate::tests_cfg::redis::setup_redis_container().await;
    let store = store(
        Arc::new(redis::Redis::connect(&url).await.unwrap()),
        SessionStoreKind::Redis,
    );
    let client = ::redis::Client::open(url.as_str()).unwrap();
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();

    // A short-lived session is indexed under its expiry ...
    let mut short = store.record(data("gina"));
    short.expires_at = now() + 1;
    insert(&store, "short", short.clone()).await;
    let score: f64 = ::redis::cmd("ZSCORE")
        .arg("loco:session:u:gina")
        .arg(hash("short"))
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!((score - short.expires_at as f64).abs() < f64::EPSILON);

    // ... and the next write for the user sweeps it once it is dead, so the
    // index cannot grow without bound.
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    put(&store, "long", "gina").await;
    let n: i64 = ::redis::cmd("ZCARD")
        .arg("loco:session:u:gina")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(n, 1);

    // The key lives exactly as long as its latest member.
    let ttl: i64 = ::redis::cmd("TTL")
        .arg("loco:session:u:gina")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!((3590..=3600).contains(&ttl), "ttl {ttl}");

    // Sliding the session moves its score and the key's expiry with it.
    let mut later = store.record(data("gina"));
    later.expires_at = now() + 9000;
    update(&store, "long", later).await;
    let ttl: i64 = ::redis::cmd("TTL")
        .arg("loco:session:u:gina")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!((8990..=9000).contains(&ttl), "ttl {ttl}");

    // A rotation (delete old id, insert new) leaves one member, not two.
    store
        .apply(vec![
            Op::Delete {
                id: "long".to_string(),
            },
            Op::Insert {
                id: "new".to_string(),
                record: store.record(data("gina")),
            },
        ])
        .await
        .unwrap();
    let n: i64 = ::redis::cmd("ZCARD")
        .arg("loco:session:u:gina")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(n, 1);

    // A sign-out empties the index, and the key goes with it.
    store
        .apply(vec![Op::Delete {
            id: "new".to_string(),
        }])
        .await
        .unwrap();
    let n: i64 = ::redis::cmd("EXISTS")
        .arg("loco:session:u:gina")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(n, 0);

    // The series index is scored by the series' fixed expiry, and rotating the
    // token does not move it: revoke-all can always find a live series.
    let (op, cookie) = store.new_remember("gina");
    store.apply(vec![op]).await.unwrap();
    let (series, first) = cookie.split_once('.').unwrap();
    let expires = store
        .backend
        .remember_get(series)
        .await
        .unwrap()
        .unwrap()
        .expires_at;
    assert!(matches!(
        store.restore(series, first).await.unwrap(),
        Restored::Ok { .. }
    ));
    let score: f64 = ::redis::cmd("ZSCORE")
        .arg("loco:session:ur:gina")
        .arg(series)
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!((score - expires as f64).abs() < f64::EPSILON);
    let ttl: i64 = ::redis::cmd("TTL")
        .arg("loco:session:ur:gina")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!((7190..=7200).contains(&ttl), "ttl {ttl}");

    // A deleted series leaves its index too.
    store
        .apply(vec![Op::RememberDelete {
            series: series.to_string(),
        }])
        .await
        .unwrap();
    let n: i64 = ::redis::cmd("EXISTS")
        .arg("loco:session:ur:gina")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[cfg(feature = "cache_redis")]
#[tokio::test]
async fn revoking_a_user_takes_every_session_and_series_in_one_step() {
    let (url, _container) = crate::tests_cfg::redis::setup_redis_container().await;
    let store = store(
        Arc::new(redis::Redis::connect(&url).await.unwrap()),
        SessionStoreKind::Redis,
    );
    for n in 0..5 {
        put(&store, &format!("s{n}"), "hank").await;
    }
    let (op, _) = store.new_remember("hank");
    store.apply(vec![op]).await.unwrap();

    // Sign-ins racing with the revoke; the revoke counts what it removed.
    let race = futures_util::future::join_all((5..10).map(|n| {
        let store = store.clone();
        async move { put(&store, &format!("s{n}"), "hank").await }
    }));
    let (_, revoked) = tokio::join!(race, store.revoke_user("hank"));
    assert!(revoked.unwrap() >= 6);
    // A sign-in that landed after the revoke is a legitimate new one; a second
    // revoke clears it and nothing is left behind.
    store.revoke_user("hank").await.unwrap();
    for n in 0..10 {
        assert!(store.load(&format!("s{n}")).await.unwrap().is_none());
    }
}

// ---- behaviour every store shares ------------------------------------------

/// A copied remember cookie is replayed: the user's sessions minted so far and
/// every one of their series go, not just the series that was replayed.
async fn theft_ends_everything_the_user_has(store: SessionStore) {
    put(&store, "t1", "ivy").await;
    put(&store, "t2", "ivy").await;
    put(&store, "t3", "jack").await;
    let (op1, first) = store.new_remember("ivy");
    let (op2, second) = store.new_remember("ivy");
    let (op3, other) = store.new_remember("jack");
    store.apply(vec![op1, op2, op3]).await.unwrap();
    let (stolen, _) = first.split_once('.').unwrap();
    let (series2, token2) = second.split_once('.').unwrap();
    let (series3, token3) = other.split_once('.').unwrap();

    assert!(matches!(
        store.restore(stolen, "not-the-token").await.unwrap(),
        Restored::Rejected
    ));
    assert!(store.load("t1").await.unwrap().is_none());
    assert!(store.load("t2").await.unwrap().is_none());
    assert!(
        matches!(
            store.restore(series2, token2).await.unwrap(),
            Restored::Rejected
        ),
        "the user's other series is revoked too"
    );
    assert!(store.load("t3").await.unwrap().is_some(), "others stay");
    assert!(matches!(
        store.restore(series3, token3).await.unwrap(),
        Restored::Ok { .. }
    ));
}

/// Remember-me is a lifetime fixed at sign-in (`remember_for`), not an idle
/// timeout: using it never extends it.
async fn rotation_keeps_the_series_expiry(store: SessionStore) {
    let (op, cookie) = store.new_remember("kim");
    store.apply(vec![op]).await.unwrap();
    let (series, first) = cookie.split_once('.').unwrap();
    let before = store.backend.remember_get(series).await.unwrap().unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let Restored::Ok {
        token: Some(second),
        ..
    } = store.restore(series, first).await.unwrap()
    else {
        panic!("valid token rejected");
    };
    let Restored::Ok { .. } = store.restore(series, &second).await.unwrap() else {
        panic!("rotated token rejected");
    };
    let after = store.backend.remember_get(series).await.unwrap().unwrap();
    assert_eq!(after.generation, 2);
    assert_eq!(after.expires_at, before.expires_at, "no sliding");
}

/// A revoke that lands while a restore is in flight must not leave the
/// restored session alive: its insert depends on the series still existing.
async fn a_restored_session_needs_its_series(store: SessionStore) {
    let (op, cookie) = store.new_remember("lee");
    store.apply(vec![op]).await.unwrap();
    let (series, _) = cookie.split_once('.').unwrap();

    assert!(store
        .insert_restored("r1", store.record(data("lee")), series)
        .await
        .unwrap());
    assert!(store.load("r1").await.unwrap().is_some());

    store.revoke_user("lee").await.unwrap();
    assert!(!store
        .insert_restored("r2", store.record(data("lee")), series)
        .await
        .unwrap());
    assert!(store.load("r2").await.unwrap().is_none());
}

macro_rules! on_every_store {
    ($($check:ident),+ $(,)?) => {$(
        mod $check {
            use super::*;

            #[cfg(feature = "with-db")]
            #[tokio::test]
            async fn database() {
                let backend = db::Database::init(sqlite().await).await.unwrap();
                super::$check(store(Arc::new(backend), SessionStoreKind::Database)).await;
            }

            #[cfg(feature = "cache_redis")]
            #[tokio::test]
            async fn redis() {
                let (url, _container) = crate::tests_cfg::redis::setup_redis_container().await;
                let backend = redis::Redis::connect(&url).await.unwrap();
                super::$check(store(Arc::new(backend), SessionStoreKind::Redis)).await;
            }
        }
    )+};
}

on_every_store!(
    theft_ends_everything_the_user_has,
    rotation_keeps_the_series_expiry,
    a_restored_session_needs_its_series,
);

#[cfg(feature = "with-db")]
#[tokio::test]
async fn user_keys_longer_than_255_characters_fit_on_postgres() {
    let (url, _container) = crate::tests_cfg::postgres::setup_postgres_database().await;
    let db = sea_orm::Database::connect(&url).await.unwrap();
    let store = store(
        Arc::new(db::Database::init(db).await.unwrap()),
        SessionStoreKind::Database,
    );
    let user = "u".repeat(400);
    put(&store, "long-user", &user).await;
    let (op, _) = store.new_remember(&user);
    store.apply(vec![op]).await.unwrap();
    assert_eq!(
        store
            .load("long-user")
            .await
            .unwrap()
            .unwrap()
            .user
            .as_deref(),
        Some(user.as_str())
    );
    assert_eq!(store.revoke_user(&user).await.unwrap(), 2);
}

#[test]
fn logs_name_a_series_by_its_hash_never_its_value() {
    let series = random_token();
    let label = log_id(&series);
    assert!(!label.contains(&series));
    assert_eq!(label, log_id(&series), "stable, so entries can be matched");
    assert_ne!(label, log_id(&random_token()));
}

// ---- configuration ----------------------------------------------------------

fn configured(f: impl FnOnce(&mut SessionConfig)) -> Config {
    let mut session = SessionConfig {
        store: SessionStoreKind::Database,
        ..Default::default()
    };
    f(&mut session);
    let mut config = crate::tests_cfg::config::test_config();
    config.auth = Some(Auth {
        session: Some(session),
        ..Default::default()
    });
    config
}

#[test]
fn a_zero_ttl_is_rejected() {
    let err = validate(&configured(|s| s.ttl = Some(0)))
        .unwrap_err()
        .to_string();
    assert!(err.contains("auth.session.ttl"), "{err}");
}

#[test]
fn a_long_ttl_needs_a_longer_remember_for_even_when_it_is_left_at_the_default() {
    let err = validate(&configured(|s| s.ttl = Some(DEFAULT_REMEMBER_FOR)))
        .unwrap_err()
        .to_string();
    assert!(err.contains("remember_for"), "{err}");
    validate(&configured(|s| s.ttl = Some(DEFAULT_REMEMBER_FOR - 1))).unwrap();
    validate(&configured(|s| {
        s.ttl = Some(DEFAULT_REMEMBER_FOR);
        s.remember_for = Some(DEFAULT_REMEMBER_FOR + 1);
    }))
    .unwrap();
    // The cookie store has no server-side ttl to compare.
    validate(&configured(|s| {
        s.store = SessionStoreKind::Cookie;
        s.ttl = Some(DEFAULT_REMEMBER_FOR);
    }))
    .unwrap();
}

#[test]
fn the_session_and_remember_cookies_need_different_names() {
    let err = validate(&configured(|s| {
        s.cookie_name = Some("sid".to_string());
        s.remember_cookie_name = Some("sid".to_string());
    }))
    .unwrap_err()
    .to_string();
    assert!(err.contains("cookie_name"), "{err}");
    let err = validate(&configured(|s| {
        s.remember_cookie_name = Some(crate::auth::cookies::DEFAULT_SESSION_COOKIE.to_string());
    }))
    .unwrap_err()
    .to_string();
    assert!(err.contains("cookie_name"), "{err}");
}

#[test]
fn remember_for_is_capped_at_what_browsers_keep() {
    const CAP: u64 = 400 * 24 * 60 * 60;
    validate(&configured(|s| s.remember_for = Some(CAP))).unwrap();
    let err = validate(&configured(|s| s.remember_for = Some(CAP + 1)))
        .unwrap_err()
        .to_string();
    assert!(err.contains("400 days"), "{err}");
}
