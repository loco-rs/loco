//! Server-side session stores and remember-me tokens.
//!
//! With `auth.session.store` set to `database` or `redis` the session cookie
//! carries only a signed, random session id (256 bits from the OS-seeded
//! CSPRNG); the data, its expiry and the owning user live on the server, so a
//! session can be ended from the server. The stores never see the raw id: they
//! are keyed by its SHA-256, so a leaked table or key dump cannot be replayed
//! as cookies.
//!
//! Remember-me follows Devise's `rememberable` shape. A separate, long-lived
//! signed cookie holds a `series` and a `token`. The server keeps only a hash
//! of the token. On a request with no live session, a valid remember cookie
//! re-establishes the session and rotates the token. A token that does not
//! match a known series means a stolen cookie was replayed, so the whole
//! series is revoked.
//!
//! You rarely touch this module directly: [`Session`](super::session::Session)
//! reads and writes through it, and [`sign_out_everywhere`] revokes a user.

use std::{collections::BTreeMap, fmt, sync::Arc};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
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
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Remember {
    pub user: String,
    /// SHA-256 of the current token.
    pub token_hash: String,
    /// SHA-256 of the token before the last rotation.
    pub prev_hash: Option<String>,
    pub rotated_at: i64,
    /// Unix seconds.
    pub expires_at: i64,
}

impl fmt::Debug for Remember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Remember")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// What a backend must provide. Session ids arrive already hashed.
#[async_trait]
pub(crate) trait Backend: Send + Sync + fmt::Debug {
    async fn load(&self, sid: &str) -> Result<Option<Record>>;
    async fn save(&self, sid: &str, record: &Record) -> Result<()>;
    async fn delete(&self, sid: &str) -> Result<()>;
    /// Deletes every session and remember series of `user`.
    async fn revoke_user(&self, user: &str) -> Result<u64>;
    async fn remember_get(&self, series: &str) -> Result<Option<Remember>>;
    async fn remember_put(&self, series: &str, remember: &Remember) -> Result<()>;
    async fn remember_delete(&self, series: &str) -> Result<()>;
    /// Deletes a batch of expired rows; stores with native TTLs do nothing.
    async fn purge_expired(&self, now: i64) -> Result<u64>;
}

/// A write to carry out once the response is built.
#[derive(Clone)]
pub(crate) enum Op {
    Save { id: String, record: Record },
    Delete { id: String },
    RememberPut { series: String, remember: Remember },
    RememberDelete { series: String },
}

/// The configured server-side store.
#[derive(Clone)]
pub struct SessionStore {
    backend: Arc<dyn Backend>,
    kind: SessionStoreKind,
    max_age: u64,
    remember_for: u64,
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
    /// Sign the user back in. `token` is the rotated token to put in the
    /// cookie, or `None` when the cookie should stay as it is.
    Ok { user: String, token: Option<String> },
    /// Unknown, expired or replayed: drop the cookie.
    Rejected,
}

impl SessionStore {
    /// Seconds a session lives, renewed on each save.
    #[must_use]
    pub const fn max_age(&self) -> u64 {
        self.max_age
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

    /// Applies `ops` in order.
    pub(crate) async fn apply(&self, ops: Vec<Op>) -> Result<()> {
        let mut saved = false;
        for op in ops {
            match op {
                Op::Save { id, record } => {
                    saved = true;
                    self.backend.save(&hash(&id), &record).await?;
                }
                Op::Delete { id } => self.backend.delete(&hash(&id)).await?,
                Op::RememberPut { series, remember } => {
                    self.backend.remember_put(&series, &remember).await?;
                }
                Op::RememberDelete { series } => self.backend.remember_delete(&series).await?,
            }
        }
        if saved && rand::rng().random_range(0..PURGE_ONE_IN) == 0 {
            // Opportunistic and best effort: never fail a request over it.
            if let Err(err) = self.backend.purge_expired(now()).await {
                tracing::warn!(error = %err, "could not purge expired sessions");
            }
        }
        Ok(())
    }

    /// A [`Record`] for `data` that expires `max_age` from now.
    pub(crate) fn record(&self, data: BTreeMap<String, Value>) -> Record {
        let user = data
            .get(super::session::USER_ID_KEY)
            .and_then(Value::as_str)
            .map(str::to_string);
        Record {
            data,
            user,
            expires_at: now().saturating_add(to_i64(self.max_age)),
        }
    }

    /// A new remember-me series for `user`: the op that stores it and the
    /// cookie value `series.token`.
    pub(crate) fn new_remember(&self, user: &str) -> (Op, String) {
        let series = random_token();
        let token = random_token();
        let remember = Remember {
            user: user.to_string(),
            token_hash: hash(&token),
            prev_hash: None,
            rotated_at: now(),
            expires_at: now().saturating_add(to_i64(self.remember_for)),
        };
        let value = format!("{series}.{token}");
        (Op::RememberPut { series, remember }, value)
    }

    /// Checks a presented remember-me cookie and rotates its token.
    pub(crate) async fn restore(&self, series: &str, token: &str) -> Result<Restored> {
        let Some(mut stored) = self.backend.remember_get(series).await? else {
            return Ok(Restored::Rejected);
        };
        let at = now();
        if stored.expires_at <= at {
            self.backend.remember_delete(series).await?;
            return Ok(Restored::Rejected);
        }
        let presented = hash(token);
        // Evaluate both so timing does not reveal which one matched.
        let current = ct_eq(&presented, &stored.token_hash);
        let previous = stored
            .prev_hash
            .as_deref()
            .is_some_and(|p| ct_eq(&presented, p));

        if current {
            let token = random_token();
            stored.prev_hash = Some(std::mem::replace(&mut stored.token_hash, hash(&token)));
            stored.rotated_at = at;
            stored.expires_at = at.saturating_add(to_i64(self.remember_for));
            self.backend.remember_put(series, &stored).await?;
            return Ok(Restored::Ok {
                user: stored.user,
                token: Some(token),
            });
        }
        if previous && at - stored.rotated_at <= ROTATION_GRACE {
            return Ok(Restored::Ok {
                user: stored.user,
                token: None,
            });
        }
        // A known series with the wrong token: the cookie was copied.
        tracing::warn!(
            series = %series,
            "remember-me token mismatch on a known series; revoking it"
        );
        self.backend.remember_delete(series).await?;
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

/// Boot-time checks and eager store creation (so a missing table or an
/// unreachable Redis fails at startup, not on the first sign-in).
///
/// # Errors
///
/// A configuration that cannot work, or a store that cannot be reached.
pub async fn init(ctx: &AppContext) -> Result<()> {
    validate(&ctx.config)?;
    if session_config(&ctx.config).is_some_and(|s| s.store.is_server_side()) {
        of(ctx).await?;
    }
    Ok(())
}

/// Rejects configurations that cannot work.
///
/// # Errors
///
/// When remember-me is configured with the `cookie` store.
pub fn validate(config: &Config) -> Result<()> {
    if let Some(session) = session_config(config)
        && session.remember_for.is_some()
        && !session.store.is_server_side()
    {
        return Err(Error::Message(
            "auth.session.remember_for is set, but remember-me needs a server-side session \
                 store to hold the hashed tokens. Set auth.session.store to `database` or \
                 `redis`, or remove remember_for."
                .to_string(),
        ));
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
    let max_age = session
        .max_age
        .unwrap_or(super::cookies::DEFAULT_SESSION_MAX_AGE);
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
            Arc::new(redis::Redis::connect(&uri, max_age.max(remember_for)).await?)
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
        max_age,
        remember_for,
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
