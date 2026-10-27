use serde_json::json;

use super::*;
use crate::config::{Auth, SessionConfig};

fn store(backend: Arc<dyn Backend>, kind: SessionStoreKind) -> SessionStore {
    SessionStore {
        backend,
        kind,
        max_age: 3600,
        remember_for: 3600,
    }
}

fn data(user: &str) -> BTreeMap<String, Value> {
    BTreeMap::from([
        (super::super::session::USER_ID_KEY.to_string(), json!(user)),
        ("cart".to_string(), json!(3)),
    ])
}

async fn save(store: &SessionStore, id: &str, record: Record) {
    store
        .apply(vec![Op::Save {
            id: id.to_string(),
            record,
        }])
        .await
        .unwrap();
}

async fn put(store: &SessionStore, id: &str, user: &str) {
    save(store, id, store.record(data(user))).await;
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
    save(&store, "sid-a", again).await;
    assert_eq!(
        store.load("sid-a").await.unwrap().unwrap().data["cart"],
        json!(4)
    );

    // expired rows read as absent
    let mut old = store.record(data("alice"));
    old.expires_at = now() - 5;
    save(&store, "sid-old", old).await;
    assert!(store.load("sid-old").await.unwrap().is_none());
    store.purge_expired().await.unwrap();

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
    let Restored::Ok { user, token } = store.restore(series, first).await.unwrap() else {
        panic!("valid token rejected");
    };
    assert_eq!(user, "carol");
    let second = token.expect("token rotates");
    assert_ne!(second, first);
    assert!(matches!(
        store.restore(series, first).await.unwrap(),
        Restored::Ok { token: None, .. }
    ));
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

    config
        .auth
        .as_mut()
        .unwrap()
        .session
        .as_mut()
        .unwrap()
        .store = SessionStoreKind::Database;
    validate(&config).unwrap();
}

#[test]
fn debug_hides_secrets() {
    let r = Remember {
        user: "u".into(),
        token_hash: "SECRETHASH".into(),
        prev_hash: None,
        rotated_at: 0,
        expires_at: 0,
    };
    assert!(!format!("{r:?}").contains("SECRETHASH"));
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
    store.backend.save(&hash("gone"), &old).await.unwrap();
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
    exercise(store(Arc::new(backend), SessionStoreKind::Database)).await;
}

#[cfg(feature = "cache_redis")]
#[tokio::test]
async fn redis_store() {
    let (url, _container) = crate::tests_cfg::redis::setup_redis_container().await;
    let backend = redis::Redis::connect(&url, 3600).await.unwrap();
    exercise(store(Arc::new(backend), SessionStoreKind::Redis)).await;
}
