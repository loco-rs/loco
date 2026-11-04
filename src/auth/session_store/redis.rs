//! The `redis` store: one key per session with a TTL, plus a per-user index of
//! session ids so "sign out everywhere" can find them.
//!
//! ```text
//! loco:session:s:<sha256(id)>      JSON Record,     EX <session ttl>
//! loco:session:u:<user>            zset of hashes,  score = expires_at
//! loco:session:r:<series>          JSON Remember,   EX <until expires_at>
//! loco:session:ur:<user>           zset of series,  score = expires_at
//! ```
//!
//! The indexes are sorted sets scored by the expiry of what they list. Every
//! write adds its member, sweeps the members that are already dead (so an
//! index cannot grow without bound) and sets the key to expire when the latest
//! member does, so revoke-all can always find whatever is still alive.
//!
//! Anything that must be one step (update-if-exists, delete-and-unindex,
//! revoke-all, compare-and-swap) is a Lua script, which Redis runs atomically.
//! The scripts declare the keys they are given. Revoking a user also deletes
//! the keys their indexes name, so this store needs a single Redis node, not
//! Redis Cluster.

use async_trait::async_trait;
use bb8_redis::{
    bb8::Pool,
    redis::{self, AsyncCommands, Script},
    RedisConnectionManager,
};

use super::{now, Backend, Record, Remember};
use crate::{Error, Result};

const PREFIX: &str = "loco:session:";

/// Prefixes a script with `index`: add a member under its expiry, sweep the
/// dead ones, and expire the key with the latest.
macro_rules! indexing {
    ($body:literal) => {
        concat!(
            r"
local function index(key, member, score, now)
  redis.call('ZADD', key, score, member)
  redis.call('ZREMRANGEBYSCORE', key, '-inf', now)
  local top = redis.call('ZRANGE', key, -1, -1, 'WITHSCORES')
  if top[2] then redis.call('EXPIREAT', key, top[2]) end
end
",
            $body
        )
    };
}

/// Stores a new session under a fresh id.
/// KEYS: session, user index. ARGV: json, ttl, member, score, now, has user.
const INSERT: &str = indexing!(
    r"
if not redis.call('SET', KEYS[1], ARGV[1], 'EX', ARGV[2], 'NX') then return 0 end
if ARGV[6] == '1' then index(KEYS[2], ARGV[3], ARGV[4], ARGV[5]) end
return 1"
);

/// Stores the session a remember-me restore minted, if its series is alive.
/// KEYS: session, user index, series. ARGV: json, ttl, member, score, now.
const INSERT_RESTORED: &str = indexing!(
    r"
if redis.call('EXISTS', KEYS[3]) == 0 then return 0 end
if not redis.call('SET', KEYS[1], ARGV[1], 'EX', ARGV[2], 'NX') then return 0 end
index(KEYS[2], ARGV[3], ARGV[4], ARGV[5])
return 1"
);

/// Rewrites a session only while it exists.
/// KEYS: session, user index. ARGV: json, ttl, member, score, now, has user.
const UPDATE: &str = indexing!(
    r"
if redis.call('EXISTS', KEYS[1]) == 0 then return 0 end
redis.call('SET', KEYS[1], ARGV[1], 'EX', ARGV[2])
if ARGV[6] == '1' then index(KEYS[2], ARGV[3], ARGV[4], ARGV[5]) end
return 1"
);

/// Stores a new remember-me series.
/// KEYS: series, user index. ARGV: json, ttl, member, score, now.
const REMEMBER_INSERT: &str = indexing!(
    r"
if not redis.call('SET', KEYS[1], ARGV[1], 'EX', ARGV[2], 'NX') then return 0 end
index(KEYS[2], ARGV[3], ARGV[4], ARGV[5])
return 1"
);

/// Replaces a series only at the expected generation.
/// KEYS: series, user index. ARGV: expected generation, json, ttl, member,
/// score, now.
const CAS: &str = indexing!(
    r"
local raw = redis.call('GET', KEYS[1])
if not raw then return 0 end
local rec = cjson.decode(raw)
if tonumber(rec.generation) ~= tonumber(ARGV[1]) then return 0 end
redis.call('SET', KEYS[1], ARGV[2], 'EX', ARGV[3])
index(KEYS[2], ARGV[4], ARGV[5], ARGV[6])
return 1"
);

/// Deletes a session or series and takes it out of its user's index (when the
/// caller knows the user).
/// KEYS: the key, optionally the user index. ARGV: member.
const DELETE: &str = r"
redis.call('DEL', KEYS[1])
if KEYS[2] then redis.call('ZREM', KEYS[2], ARGV[1]) end
return 1";

/// Deletes every session and series a user owns, as one step. The keys it
/// deletes are named by the indexes, so they cannot be declared up front.
/// KEYS: session index, series index. ARGV: key prefix.
const REVOKE: &str = r"
local n = 0
for _, id in ipairs(redis.call('ZRANGE', KEYS[1], 0, -1)) do
  n = n + redis.call('DEL', ARGV[1] .. 's:' .. id)
end
for _, id in ipairs(redis.call('ZRANGE', KEYS[2], 0, -1)) do
  n = n + redis.call('DEL', ARGV[1] .. 'r:' .. id)
end
redis.call('DEL', KEYS[1], KEYS[2])
return n";

#[derive(Debug)]
pub(super) struct Redis {
    pool: Pool<RedisConnectionManager>,
}

fn err(e: impl std::fmt::Display) -> Error {
    Error::Message(format!("session redis store: {e}"))
}

impl Redis {
    pub(super) async fn connect(uri: &str) -> Result<Self> {
        let manager = RedisConnectionManager::new(uri).map_err(err)?;
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .await
            .map_err(err)?;
        let mut conn = pool.get().await.map_err(err)?;
        redis::cmd("PING")
            .query_async::<()>(&mut *conn)
            .await
            .map_err(err)?;
        drop(conn);
        Ok(Self { pool })
    }

    async fn conn(&self) -> Result<bb8_redis::bb8::PooledConnection<'_, RedisConnectionManager>> {
        self.pool.get().await.map_err(err)
    }

    /// The user a stored JSON value names, for un-indexing it on delete.
    async fn owner(&self, key: &str) -> Result<Option<String>> {
        let raw: Option<String> = self.conn().await?.get(key).await.map_err(err)?;
        Ok(raw
            .and_then(|r| serde_json::from_str::<serde_json::Value>(&r).ok())
            .and_then(|v| v.get("user").and_then(|u| u.as_str().map(str::to_string))))
    }

    async fn delete_key(&self, key: String, index_prefix: &str, member: &str) -> Result<()> {
        let owner = self.owner(&key).await?;
        let script = Script::new(DELETE);
        let mut call = script.key(&key);
        if let Some(user) = owner {
            call.key(format!("{PREFIX}{index_prefix}:{user}"));
        }
        call.arg(member)
            .invoke_async::<i64>(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(())
    }
}

fn ttl(expires_at: i64) -> i64 {
    (expires_at - now()).max(1)
}

fn user_index(user: &str) -> String {
    format!("{PREFIX}u:{user}")
}

fn series_index(user: &str) -> String {
    format!("{PREFIX}ur:{user}")
}

#[async_trait]
impl Backend for Redis {
    async fn load(&self, sid: &str) -> Result<Option<Record>> {
        let raw: Option<String> = self
            .conn()
            .await?
            .get(format!("{PREFIX}s:{sid}"))
            .await
            .map_err(err)?;
        raw.map(|r| serde_json::from_str(&r).map_err(Into::into))
            .transpose()
    }

    async fn insert(&self, sid: &str, record: &Record) -> Result<()> {
        Script::new(INSERT)
            .key(format!("{PREFIX}s:{sid}"))
            .key(user_index(record.user.as_deref().unwrap_or("")))
            .arg(serde_json::to_string(record)?)
            .arg(ttl(record.expires_at))
            .arg(sid)
            .arg(record.expires_at)
            .arg(now())
            .arg(if record.user.is_some() { "1" } else { "0" })
            .invoke_async::<i64>(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(())
    }

    async fn insert_restored(&self, sid: &str, record: &Record, series: &str) -> Result<bool> {
        let user = record.user.as_deref().unwrap_or("");
        let stored: i64 = Script::new(INSERT_RESTORED)
            .key(format!("{PREFIX}s:{sid}"))
            .key(user_index(user))
            .key(format!("{PREFIX}r:{series}"))
            .arg(serde_json::to_string(record)?)
            .arg(ttl(record.expires_at))
            .arg(sid)
            .arg(record.expires_at)
            .arg(now())
            .invoke_async(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(stored == 1)
    }

    async fn update(&self, sid: &str, record: &Record) -> Result<bool> {
        let updated: i64 = Script::new(UPDATE)
            .key(format!("{PREFIX}s:{sid}"))
            .key(user_index(record.user.as_deref().unwrap_or("")))
            .arg(serde_json::to_string(record)?)
            .arg(ttl(record.expires_at))
            .arg(sid)
            .arg(record.expires_at)
            .arg(now())
            .arg(if record.user.is_some() { "1" } else { "0" })
            .invoke_async(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(updated == 1)
    }

    async fn delete(&self, sid: &str) -> Result<()> {
        self.delete_key(format!("{PREFIX}s:{sid}"), "u", sid).await
    }

    async fn revoke_user(&self, user: &str) -> Result<u64> {
        let n: i64 = Script::new(REVOKE)
            .key(user_index(user))
            .key(series_index(user))
            .arg(PREFIX)
            .invoke_async(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    async fn remember_get(&self, series: &str) -> Result<Option<Remember>> {
        let raw: Option<String> = self
            .conn()
            .await?
            .get(format!("{PREFIX}r:{series}"))
            .await
            .map_err(err)?;
        raw.map(|r| serde_json::from_str(&r).map_err(Into::into))
            .transpose()
    }

    async fn remember_insert(&self, series: &str, r: &Remember) -> Result<()> {
        Script::new(REMEMBER_INSERT)
            .key(format!("{PREFIX}r:{series}"))
            .key(series_index(&r.user))
            .arg(serde_json::to_string(r)?)
            .arg(ttl(r.expires_at))
            .arg(series)
            .arg(r.expires_at)
            .arg(now())
            .invoke_async::<i64>(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(())
    }

    async fn remember_cas(
        &self,
        series: &str,
        expected_generation: i64,
        r: &Remember,
    ) -> Result<bool> {
        let swapped: i64 = Script::new(CAS)
            .key(format!("{PREFIX}r:{series}"))
            .key(series_index(&r.user))
            .arg(expected_generation)
            .arg(serde_json::to_string(r)?)
            .arg(ttl(r.expires_at))
            .arg(series)
            .arg(r.expires_at)
            .arg(now())
            .invoke_async(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(swapped == 1)
    }

    async fn remember_delete(&self, series: &str) -> Result<()> {
        self.delete_key(format!("{PREFIX}r:{series}"), "ur", series)
            .await
    }

    async fn purge_expired(&self, _now: i64) -> Result<u64> {
        // Keys carry a TTL; there is nothing to sweep.
        Ok(0)
    }
}
