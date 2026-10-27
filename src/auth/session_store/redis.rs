//! The `redis` store: one key per session with a TTL, plus a per-user set of
//! session ids so "sign out everywhere" can find them.
//!
//! ```text
//! loco:session:s:<sha256(id)>      JSON Record,     EX <session ttl>
//! loco:session:u:<user>            set of hashes,   EX <longest ttl>
//! loco:session:r:<series>          JSON Remember,   EX <remember ttl>
//! loco:session:ur:<user>           set of series,   EX <longest ttl>
//! ```

use async_trait::async_trait;
use bb8_redis::{
    bb8::Pool,
    redis::{self, AsyncCommands},
    RedisConnectionManager,
};

use super::{now, Backend, Record, Remember};
use crate::{Error, Result};

const PREFIX: &str = "loco:session:";

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
        // Fail at boot, not on the first sign-in.
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

    async fn save(&self, sid: &str, record: &Record) -> Result<()> {
        let key = format!("{PREFIX}s:{sid}");
        let mut pipe = redis::pipe();
        pipe.set_ex(
            &key,
            serde_json::to_string(record)?,
            ttl(record.expires_at) as u64,
        )
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

    async fn delete(&self, sid: &str) -> Result<()> {
        self.conn()
            .await?
            .del::<_, ()>(format!("{PREFIX}s:{sid}"))
            .await
            .map_err(err)
    }

    async fn revoke_user(&self, user: &str) -> Result<u64> {
        let mut conn = self.conn().await?;
        let mut count = 0;
        for (set, kind) in [
            (format!("{PREFIX}u:{user}"), "s"),
            (format!("{PREFIX}ur:{user}"), "r"),
        ] {
            let members: Vec<String> = conn.smembers(&set).await.map_err(err)?;
            let mut pipe = redis::pipe();
            for member in &members {
                pipe.del(format!("{PREFIX}{kind}:{member}")).ignore();
            }
            pipe.del(&set).ignore();
            pipe.query_async::<()>(&mut *conn).await.map_err(err)?;
            count += members.len() as u64;
        }
        Ok(count)
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

    async fn remember_put(&self, series: &str, r: &Remember) -> Result<()> {
        let set = format!("{PREFIX}ur:{}", r.user);
        let mut pipe = redis::pipe();
        pipe.set_ex(
            format!("{PREFIX}r:{series}"),
            serde_json::to_string(r)?,
            ttl(r.expires_at) as u64,
        )
        .ignore()
        .sadd(&set, series)
        .ignore()
        .expire(&set, self.set_ttl)
        .ignore();
        pipe.query_async::<()>(&mut *self.conn().await?)
            .await
            .map_err(err)
    }

    async fn remember_delete(&self, series: &str) -> Result<()> {
        self.conn()
            .await?
            .del::<_, ()>(format!("{PREFIX}r:{series}"))
            .await
            .map_err(err)
    }

    async fn purge_expired(&self, _now: i64) -> Result<u64> {
        // Keys carry a TTL; there is nothing to sweep.
        Ok(0)
    }
}
