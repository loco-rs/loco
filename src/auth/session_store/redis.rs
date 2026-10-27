//! The `redis` store: one key per session with a TTL, plus a per-user set of
//! session ids so "sign out everywhere" can find them.
//!
//! ```text
//! loco:session:s:<sha256(id)>      JSON Record,     EX <session ttl>
//! loco:session:u:<user>            set of hashes,   EX <longest ttl>
//! loco:session:r:<series>          JSON Remember,   EX <remember ttl>
//! loco:session:ur:<user>           set of series,   EX <longest ttl>
//! ```
//!
//! Anything that must be one step (update-if-exists, delete-and-unindex,
//! revoke-all, compare-and-swap) is a Lua script, which Redis runs atomically.

use async_trait::async_trait;
use bb8_redis::{
    bb8::Pool,
    redis::{self, AsyncCommands, Script},
    RedisConnectionManager,
};

use super::{now, Backend, Record, Remember};
use crate::{Error, Result};

const PREFIX: &str = "loco:session:";

/// Rewrites a session only while it exists.
/// KEYS: session, user set. ARGV: json, ttl, member, set ttl, has user.
const UPDATE: &str = r"
if redis.call('EXISTS', KEYS[1]) == 0 then return 0 end
redis.call('SET', KEYS[1], ARGV[1], 'EX', ARGV[2])
if ARGV[5] == '1' then
  redis.call('SADD', KEYS[2], ARGV[3])
  redis.call('EXPIRE', KEYS[2], ARGV[4])
end
return 1";

/// Deletes a session or series and takes it out of its user's set.
/// KEYS: the key. ARGV: set key prefix, member.
const DELETE: &str = r"
local raw = redis.call('GET', KEYS[1])
if raw then
  local ok, rec = pcall(cjson.decode, raw)
  if ok and type(rec) == 'table' and type(rec.user) == 'string' then
    redis.call('SREM', ARGV[1] .. rec.user, ARGV[2])
  end
  redis.call('DEL', KEYS[1])
end
return 1";

/// Deletes every session and series a user owns, as one step.
/// KEYS: session set, series set. ARGV: key prefix.
const REVOKE: &str = r"
local n = 0
for _, id in ipairs(redis.call('SMEMBERS', KEYS[1])) do
  n = n + redis.call('DEL', ARGV[1] .. 's:' .. id)
end
for _, id in ipairs(redis.call('SMEMBERS', KEYS[2])) do
  n = n + redis.call('DEL', ARGV[1] .. 'r:' .. id)
end
redis.call('DEL', KEYS[1], KEYS[2])
return n";

/// Replaces a series only at the expected generation.
/// KEYS: series. ARGV: expected generation, json, ttl.
const CAS: &str = r"
local raw = redis.call('GET', KEYS[1])
if not raw then return 0 end
local rec = cjson.decode(raw)
if tonumber(rec.generation) ~= tonumber(ARGV[1]) then return 0 end
redis.call('SET', KEYS[1], ARGV[2], 'EX', ARGV[3])
return 1";

#[derive(Debug)]
pub(super) struct Redis {
    pool: Pool<RedisConnectionManager>,
    /// Longest lifetime of anything a user set refers to, in seconds.
    set_ttl: i64,
}

fn err(e: impl std::fmt::Display) -> Error {
    Error::Message(format!("session redis store: {e}"))
}

impl Redis {
    pub(super) async fn connect(uri: &str, set_ttl: u64) -> Result<Self> {
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
        Ok(Self {
            pool,
            set_ttl: super::to_i64(set_ttl),
        })
    }

    async fn conn(&self) -> Result<bb8_redis::bb8::PooledConnection<'_, RedisConnectionManager>> {
        self.pool.get().await.map_err(err)
    }
}

fn ttl(expires_at: i64) -> i64 {
    (expires_at - now()).max(1)
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
        let key = format!("{PREFIX}s:{sid}");
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("SET")
            .arg(&key)
            .arg(serde_json::to_string(record)?)
            .arg("EX")
            .arg(ttl(record.expires_at))
            .arg("NX")
            .ignore();
        if let Some(user) = &record.user {
            let set = format!("{PREFIX}u:{user}");
            pipe.sadd(&set, sid)
                .ignore()
                .expire(&set, self.set_ttl)
                .ignore();
        }
        pipe.query_async::<()>(&mut *self.conn().await?)
            .await
            .map_err(err)
    }

    async fn update(&self, sid: &str, record: &Record) -> Result<bool> {
        let set = format!("{PREFIX}u:{}", record.user.as_deref().unwrap_or(""));
        let updated: i64 = Script::new(UPDATE)
            .key(format!("{PREFIX}s:{sid}"))
            .key(set)
            .arg(serde_json::to_string(record)?)
            .arg(ttl(record.expires_at))
            .arg(sid)
            .arg(self.set_ttl)
            .arg(if record.user.is_some() { "1" } else { "0" })
            .invoke_async(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(updated == 1)
    }

    async fn delete(&self, sid: &str) -> Result<()> {
        Script::new(DELETE)
            .key(format!("{PREFIX}s:{sid}"))
            .arg(format!("{PREFIX}u:"))
            .arg(sid)
            .invoke_async::<i64>(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(())
    }

    async fn revoke_user(&self, user: &str) -> Result<u64> {
        let n: i64 = Script::new(REVOKE)
            .key(format!("{PREFIX}u:{user}"))
            .key(format!("{PREFIX}ur:{user}"))
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
        let set = format!("{PREFIX}ur:{}", r.user);
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("SET")
            .arg(format!("{PREFIX}r:{series}"))
            .arg(serde_json::to_string(r)?)
            .arg("EX")
            .arg(ttl(r.expires_at))
            .arg("NX")
            .ignore()
            .sadd(&set, series)
            .ignore()
            .expire(&set, self.set_ttl)
            .ignore();
        pipe.query_async::<()>(&mut *self.conn().await?)
            .await
            .map_err(err)
    }

    async fn remember_cas(
        &self,
        series: &str,
        expected_generation: i64,
        r: &Remember,
    ) -> Result<bool> {
        let swapped: i64 = Script::new(CAS)
            .key(format!("{PREFIX}r:{series}"))
            .arg(expected_generation)
            .arg(serde_json::to_string(r)?)
            .arg(ttl(r.expires_at))
            .invoke_async(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(swapped == 1)
    }

    async fn remember_delete(&self, series: &str) -> Result<()> {
        Script::new(DELETE)
            .key(format!("{PREFIX}r:{series}"))
            .arg(format!("{PREFIX}ur:"))
            .arg(series)
            .invoke_async::<i64>(&mut *self.conn().await?)
            .await
            .map_err(err)?;
        Ok(())
    }

    async fn purge_expired(&self, _now: i64) -> Result<u64> {
        // Keys carry a TTL; there is nothing to sweep.
        Ok(0)
    }
}
