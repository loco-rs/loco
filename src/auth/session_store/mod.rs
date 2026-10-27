//! Server-side session stores and remember-me tokens.
//!
//! With `auth.session.store` set to `database` or `redis` the session cookie
//! carries only a signed, random session id (256 bits from the OS-seeded
//! CSPRNG); the data, its expiry and the owning user live on the server, so a
//! session can be ended from the server. The stores never see the raw id: they
//! are keyed by its SHA-256, so a leaked table or key dump cannot be replayed
//! as cookies.
//!
//! A server-side session lives `auth.session.ttl` (default one day, sliding)
//! and its cookie has no `Max-Age`: it ends with the browser, and only
//! remember-me outlives that.
//!
//! Remember-me follows Devise's `rememberable` shape. A separate, long-lived
//! signed cookie holds a `series` and a `token`. The server keeps the series,
//! the user and a generation counter; the token of generation `g` is
//! `HMAC-SHA256(cookie signing key, series | g)`, so a database dump alone
//! cannot mint one, and the current token can be handed out again (see
//! [`SessionStore::restore`]). On a request with no live session, a valid
//! remember cookie re-establishes the session and rotates the token by
//! bumping the generation, with a compare-and-swap so parallel requests agree
//! on one winner. A token that is neither the current nor the just-replaced
//! one means a stolen cookie was replayed, so the whole series is revoked.
//!
//! You rarely touch this module directly: [`Session`](super::session::Session)
//! reads and writes through it, and [`sign_out_everywhere`] revokes a user.

use std::{collections::BTreeMap, fmt, sync::Arc};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    app::AppContext,
    config::{Config, SessionStoreKind},
    Error, Result,
};

#[cfg(feature = "with-db")]
mod db;
#[cfg(feature = "cache_redis")]
mod redis;
#[cfg(test)]
mod tests;

/// Idle lifetime of a server-side session when `auth.session.ttl` is unset: a
/// day.
pub const DEFAULT_TTL: u64 = 60 * 60 * 24;
/// Remember-me lifetime when `auth.session.remember_for` is unset: 14 days.
pub const DEFAULT_REMEMBER_FOR: u64 = 60 * 60 * 24 * 14;
/// Default remember-me cookie name.
pub const DEFAULT_REMEMBER_COOKIE: &str = "loco_remember";

/// After a remember token rotates, the previous one is still accepted this
/// long (seconds). A browser fires several requests at once; without this the
/// slower ones would look like a replayed, stolen cookie.
const ROTATION_GRACE: i64 = 10;

/// One in this many session writes also deletes a batch of expired rows.
const PURGE_ONE_IN: u32 = 64;

/// A stored session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Record {
    pub data: BTreeMap<String, Value>,
    pub user: Option<String>,
    /// Unix seconds.
    pub expires_at: i64,
}

/// A stored remember-me series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Remember {
    pub user: String,
    /// Bumped on every rotation; the current token derives from it.
    pub generation: i64,
    pub rotated_at: i64,
    /// Unix seconds.
    pub expires_at: i64,
}

/// What a backend must provide. Session ids arrive already hashed.
#[async_trait]
pub(crate) trait Backend: Send + Sync + fmt::Debug {
    async fn load(&self, sid: &str) -> Result<Option<Record>>;
    /// Stores a session under a freshly minted id; fails if it exists.
    async fn insert(&self, sid: &str, record: &Record) -> Result<()>;
    /// Rewrites an existing session. `false` when it is gone (signed out,
    /// revoked or expired in the meantime): it is never recreated.
    async fn update(&self, sid: &str, record: &Record) -> Result<bool>;
    async fn delete(&self, sid: &str) -> Result<()>;
    /// Deletes every session and remember series of `user`, atomically.
    async fn revoke_user(&self, user: &str) -> Result<u64>;
    async fn remember_get(&self, series: &str) -> Result<Option<Remember>>;
    async fn remember_insert(&self, series: &str, remember: &Remember) -> Result<()>;
    /// Replaces the series only if it is still at `expected_generation`.
    async fn remember_cas(
        &self,
        series: &str,
        expected_generation: i64,
        remember: &Remember,
    ) -> Result<bool>;
    async fn remember_delete(&self, series: &str) -> Result<()>;
    /// Deletes a batch of expired rows; stores with native TTLs do nothing.
    async fn purge_expired(&self, now: i64) -> Result<u64>;

    /// Carries out `ops` in order, with hashed ids. Stores that can make them
    /// atomic override this; a lost [`Op::Update`] is not an error.
    async fn apply(&self, ops: &[Op]) -> Result<()> {
        for op in ops {
            match op {
                Op::Insert { id, record } => self.insert(id, record).await?,
                Op::Update { id, record } => {
                    self.update(id, record).await?;
                }
                Op::Delete { id } => self.delete(id).await?,
                Op::RememberInsert { series, remember } => {
                    self.remember_insert(series, remember).await?;
                }
                Op::RememberDelete { series } => self.remember_delete(series).await?,
            }
        }
        Ok(())
    }
}

/// A write to carry out once the response is built.
#[derive(Clone)]
pub(crate) enum Op {
    /// A session under a freshly minted id.
    Insert {
        id: String,
        record: Record,
    },
    /// A session that existed when the request began. Never resurrects.
    Update {
        id: String,
        record: Record,
    },
    Delete {
        id: String,
    },
    RememberInsert {
        series: String,
        remember: Remember,
    },
    RememberDelete {
        series: String,
    },
}

/// The configured server-side store.
#[derive(Clone)]
pub struct SessionStore {
    backend: Arc<dyn Backend>,
    kind: SessionStoreKind,
    ttl: u64,
    remember_for: u64,
    /// Keys the remember-me tokens (the cookie signing key).
    secret: Vec<u8>,
}

impl fmt::Debug for SessionStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionStore")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// The outcome of presenting a remember-me cookie.
pub(crate) enum Restored {
    /// Sign the user back in. `token` is the token to put in the cookie.
    Ok { user: String, token: Option<String> },
    /// Unknown, expired or replayed: drop the cookie.
    Rejected,
}

impl SessionStore {
    /// Seconds an idle session lives; it slides on use.
    #[must_use]
    pub const fn ttl(&self) -> u64 {
        self.ttl
    }

    /// Seconds a remember-me series lives.
    #[must_use]
    pub const fn remember_for(&self) -> u64 {
        self.remember_for
    }

    /// Which store this is.
    #[must_use]
    pub const fn kind(&self) -> SessionStoreKind {
        self.kind
    }

    /// The session `id`, unless unknown, revoked or expired.
    pub(crate) async fn load(&self, id: &str) -> Result<Option<Record>> {
        let record = self.backend.load(&hash(id)).await?;
        Ok(record.filter(|r| r.expires_at > now()))
    }

    /// Applies `ops` in order (atomically where the store can).
    pub(crate) async fn apply(&self, ops: Vec<Op>) -> Result<()> {
        let hashed: Vec<Op> = ops
            .into_iter()
            .map(|op| match op {
                Op::Insert { id, record } => Op::Insert {
                    id: hash(&id),
                    record,
                },
                Op::Update { id, record } => Op::Update {
                    id: hash(&id),
                    record,
                },
                Op::Delete { id } => Op::Delete { id: hash(&id) },
                other => other,
            })
            .collect();
        let wrote = hashed
            .iter()
            .any(|op| matches!(op, Op::Insert { .. } | Op::Update { .. }));
        self.backend.apply(&hashed).await?;
        if wrote && rand::rng().random_range(0..PURGE_ONE_IN) == 0 {
            // Opportunistic and best effort: never fail a request over it.
            if let Err(err) = self.backend.purge_expired(now()).await {
                tracing::warn!(error = %err, "could not purge expired sessions");
            }
        }
        Ok(())
    }

    /// A [`Record`] for `data` that expires `ttl` from now.
    pub(crate) fn record(&self, data: BTreeMap<String, Value>) -> Record {
        let user = data
            .get(super::session::USER_ID_KEY)
            .and_then(Value::as_str)
            .map(str::to_string);
        Record {
            data,
            user,
            expires_at: now().saturating_add(to_i64(self.ttl)),
        }
    }

    /// The token of `generation` in `series`.
    fn token(&self, series: &str, generation: i64) -> String {
        // HMAC accepts keys of any length, so this cannot fail.
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret)
            .unwrap_or_else(|_| unreachable!("HMAC takes any key length"));
        mac.update(b"loco-remember-v1\0");
        mac.update(series.as_bytes());
        mac.update(b"\0");
        mac.update(&generation.to_be_bytes());
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    }

    /// A new remember-me series for `user`: the op that stores it and the
    /// cookie value `series.token`.
    pub(crate) fn new_remember(&self, user: &str) -> (Op, String) {
        let series = random_token();
        let remember = Remember {
            user: user.to_string(),
            generation: 0,
            rotated_at: now(),
            expires_at: now().saturating_add(to_i64(self.remember_for)),
        };
        let value = format!("{series}.{}", self.token(&series, 0));
        (Op::RememberInsert { series, remember }, value)
    }

    /// Checks a presented remember-me cookie and rotates its token.
    ///
    /// Browsers send several requests at once, and a response carrying the
    /// rotated cookie can be lost. So the token just replaced stays valid for
    /// [`ROTATION_GRACE`] seconds, and is answered with the *current* token
    /// (never a new rotation): whichever response the browser keeps, its
    /// cookie works. Rotation is a compare-and-swap on the generation, so
    /// parallel requests with the same token produce one rotation and the
    /// rest take the grace path. Anything else on a known series is a copied
    /// cookie and revokes the series.
    pub(crate) async fn restore(&self, series: &str, token: &str) -> Result<Restored> {
        // A few lost races at most; each loss means someone else rotated.
        for _ in 0..3 {
            let Some(stored) = self.backend.remember_get(series).await? else {
                return Ok(Restored::Rejected);
            };
            let at = now();
            if stored.expires_at <= at {
                self.backend.remember_delete(series).await?;
                return Ok(Restored::Rejected);
            }
            let generation = stored.generation;
            let current = self.token(series, generation);
            let previous = (generation > 0).then(|| self.token(series, generation - 1));
            // Evaluate both so timing does not reveal which one matched.
            let is_current = ct_eq(token, &current);
            let is_previous = previous.as_deref().is_some_and(|p| ct_eq(token, p));

            if is_current {
                let rotated = Remember {
                    user: stored.user.clone(),
                    generation: generation + 1,
                    rotated_at: at,
                    expires_at: at.saturating_add(to_i64(self.remember_for)),
                };
                if self
                    .backend
                    .remember_cas(series, generation, &rotated)
                    .await?
                {
                    return Ok(Restored::Ok {
                        user: stored.user,
                        token: Some(self.token(series, generation + 1)),
                    });
                }
                continue;
            }
            if is_previous && at - stored.rotated_at <= ROTATION_GRACE {
                return Ok(Restored::Ok {
                    user: stored.user,
                    token: Some(current),
                });
            }
            // A known series with the wrong token: the cookie was copied.
            tracing::warn!(
                series = %series,
                "remember-me token mismatch on a known series; revoking it"
            );
            self.backend.remember_delete(series).await?;
            return Ok(Restored::Rejected);
        }
        Ok(Restored::Rejected)
    }

    /// Ends every session and remember-me series of `user`.
    ///
    /// # Errors
    ///
    /// When the store cannot be reached.
    pub async fn revoke_user(&self, user: &str) -> Result<u64> {
        self.backend.revoke_user(user).await
    }

    /// Deletes expired rows now (a no-op for Redis, which expires keys
    /// itself). Handy for a scheduled task.
    ///
    /// # Errors
    ///
    /// When the store cannot be reached.
    pub async fn purge_expired(&self) -> Result<u64> {
        self.backend.purge_expired(now()).await
    }
}

/// Ends every session and remember-me token of `user_id`: "sign out of all
/// devices". The user's other browsers are signed out on their next request.
///
/// ```rust,ignore
/// async fn sign_out_everywhere(
///     State(ctx): State<AppContext>,
///     CurrentUser(user): CurrentUser<users::Model>,
///     session: Session,
/// ) -> Result<impl IntoResponse> {
///     auth::session_store::sign_out_everywhere(&ctx, &user.pid.to_string()).await?;
///     Ok((session.clear(), format::redirect("/")?))
/// }
/// ```
///
/// # Errors
///
/// When `auth.session.store` is `cookie` (a cookie cannot be revoked), or the
/// store cannot be reached.
pub async fn sign_out_everywhere(ctx: &AppContext, user_id: &str) -> Result<u64> {
    of(ctx).await?.revoke_user(user_id).await
}

/// The store `ctx` is configured with, built on first use.
///
/// # Errors
///
/// When the store is `cookie`, not compiled in, or cannot be reached.
pub async fn of(ctx: &AppContext) -> Result<Arc<SessionStore>> {
    if let Some(store) = ctx.shared_store.get::<Arc<SessionStore>>() {
        return Ok(store);
    }
    let store = Arc::new(build(ctx).await?);
    ctx.shared_store.insert(store.clone());
    Ok(store)
}

/// Whether `ctx` is configured with a server-side session store.
#[must_use]
pub fn is_server_side(ctx: &AppContext) -> bool {
    session_config(&ctx.config).is_some_and(|s| s.store.is_server_side())
}

/// Rejects configurations that cannot work. Checked at boot; the store
/// itself is only built (and its tables created) on first use, so commands
/// that never touch a session do not need the database or Redis for it.
///
/// # Errors
///
/// When remember-me is configured with the `cookie` store, or would expire
/// before the session it is meant to outlive.
pub fn validate(config: &Config) -> Result<()> {
    let Some(session) = session_config(config) else {
        return Ok(());
    };
    let Some(remember_for) = session.remember_for else {
        return Ok(());
    };
    if !session.store.is_server_side() {
        return Err(Error::Message(
            "auth.session.remember_for is set, but remember-me needs a server-side session \
             store to hold its series. Set auth.session.store to `database` or `redis`, or \
             remove remember_for."
                .to_string(),
        ));
    }
    let ttl = session.ttl.unwrap_or(DEFAULT_TTL);
    if remember_for <= ttl {
        return Err(Error::Message(format!(
            "auth.session.remember_for ({remember_for}s) must be longer than auth.session.ttl \
             ({ttl}s): a remember-me cookie that expires with the session never restores one"
        )));
    }
    Ok(())
}

fn session_config(config: &Config) -> Option<&crate::config::SessionConfig> {
    config.auth.as_ref().and_then(|a| a.session.as_ref())
}

#[cfg_attr(
    not(any(feature = "with-db", feature = "cache_redis")),
    allow(unused_variables, unreachable_code)
)]
async fn build(ctx: &AppContext) -> Result<SessionStore> {
    validate(&ctx.config)?;
    let session = session_config(&ctx.config).cloned().unwrap_or_default();
    let ttl = session.ttl.unwrap_or(DEFAULT_TTL);
    let remember_for = session.remember_for.unwrap_or(DEFAULT_REMEMBER_FOR);
    let backend: Arc<dyn Backend> = match session.store {
        SessionStoreKind::Cookie => {
            return Err(Error::Message(
                "this needs a server-side session store; set auth.session.store to `database` \
                 or `redis`"
                    .to_string(),
            ))
        }
        #[cfg(feature = "with-db")]
        SessionStoreKind::Database => Arc::new(db::Database::init(ctx.db.clone()).await?),
        #[cfg(not(feature = "with-db"))]
        SessionStoreKind::Database => {
            return Err(Error::Message(
                "auth.session.store is `database`, but loco-rs was built without `with-db`"
                    .to_string(),
            ))
        }
        #[cfg(feature = "cache_redis")]
        SessionStoreKind::Redis => {
            let uri = session
                .redis_uri
                .clone()
                .or_else(|| match &ctx.config.cache {
                    crate::config::CacheConfig::Redis(r) => Some(r.uri.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    Error::Message(
                        "auth.session.store is `redis` but neither auth.session.redis_uri nor a \
                         redis cache is configured"
                            .to_string(),
                    )
                })?;
            Arc::new(redis::Redis::connect(&uri, ttl.max(remember_for)).await?)
        }
        #[cfg(not(feature = "cache_redis"))]
        SessionStoreKind::Redis => {
            return Err(Error::Message(
                "auth.session.store is `redis`, but loco-rs was built without `cache_redis`"
                    .to_string(),
            ))
        }
    };
    Ok(SessionStore {
        backend,
        kind: session.store,
        ttl,
        remember_for,
        secret: super::cookies::key(ctx)?.signing().to_vec(),
    })
}

/// A fresh 256-bit random value, base64url. `rand::rng()` is a CSPRNG seeded
/// from the operating system.
#[must_use]
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// The key a session id is stored under.
pub(crate) fn hash(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(value.as_bytes()))
}

/// Equality that does not stop at the first differing byte.
pub(crate) fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub(crate) fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

pub(crate) fn to_i64(seconds: u64) -> i64 {
    i64::try_from(seconds).unwrap_or(i64::MAX)
}
