//! The `database` store: `loco_sessions` and `loco_remember_tokens`, created
//! idempotently at startup (the way the Postgres and `SQLite` queues create
//! theirs), on Postgres and `SQLite`.

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

use super::{Backend, Record, Remember};
use crate::{Error, Result};

/// Rows deleted per purge.
const PURGE_BATCH: u32 = 500;

#[derive(Debug)]
pub(super) struct Database {
    db: DatabaseConnection,
    backend: DbBackend,
}

impl Database {
    pub(super) async fn init(db: DatabaseConnection) -> Result<Self> {
        let backend = db.get_database_backend();
        if matches!(backend, DbBackend::MySql) {
            return Err(Error::Message(
                "auth.session.store `database` supports Postgres and SQLite".to_string(),
            ));
        }
        let this = Self { db, backend };
        for ddl in [
            "CREATE TABLE IF NOT EXISTS loco_sessions (id_hash VARCHAR(64) PRIMARY KEY, user_key \
             VARCHAR(255), data TEXT NOT NULL, expires_at BIGINT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS loco_sessions_user_key_idx ON loco_sessions (user_key)",
            "CREATE INDEX IF NOT EXISTS loco_sessions_expires_at_idx ON loco_sessions \
             (expires_at)",
            "CREATE TABLE IF NOT EXISTS loco_remember_tokens (series VARCHAR(64) PRIMARY KEY, \
             user_key VARCHAR(255) NOT NULL, token_hash VARCHAR(64) NOT NULL, prev_hash \
             VARCHAR(64), rotated_at BIGINT NOT NULL, expires_at BIGINT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS loco_remember_tokens_user_key_idx ON \
             loco_remember_tokens (user_key)",
        ] {
            this.exec(ddl, vec![]).await?;
        }
        Ok(this)
    }

    /// Statements are written with `?`; Postgres wants `$1, $2, ...`.
    fn statement(&self, sql: &str, values: Vec<Value>) -> Statement {
        let sql = if matches!(self.backend, DbBackend::Postgres) {
            let mut n = 0;
            let mut out = String::with_capacity(sql.len() + 8);
            for c in sql.chars() {
                if c == '?' {
                    n += 1;
                    out.push_str(&format!("${n}"));
                } else {
                    out.push(c);
                }
            }
            out
        } else {
            sql.to_string()
        };
        Statement::from_sql_and_values(self.backend, sql, values)
    }

    async fn exec(&self, sql: &str, values: Vec<Value>) -> Result<u64> {
        Ok(self
            .db
            .execute_raw(self.statement(sql, values))
            .await?
            .rows_affected())
    }
}

#[async_trait]
impl Backend for Database {
    async fn load(&self, sid: &str) -> Result<Option<Record>> {
        let row = self
            .db
            .query_one_raw(self.statement(
                "SELECT user_key, data, expires_at FROM loco_sessions WHERE id_hash = ?",
                vec![sid.into()],
            ))
            .await?;
        let Some(row) = row else { return Ok(None) };
        let data: String = row.try_get("", "data")?;
        Ok(Some(Record {
            user: row.try_get("", "user_key")?,
            data: serde_json::from_str(&data)?,
            expires_at: row.try_get("", "expires_at")?,
        }))
    }

    async fn save(&self, sid: &str, record: &Record) -> Result<()> {
        self.exec(
            "INSERT INTO loco_sessions (id_hash, user_key, data, expires_at) VALUES (?, ?, ?, ?) \
             ON CONFLICT (id_hash) DO UPDATE SET user_key = excluded.user_key, data = \
             excluded.data, expires_at = excluded.expires_at",
            vec![
                sid.into(),
                record.user.clone().into(),
                serde_json::to_string(&record.data)?.into(),
                record.expires_at.into(),
            ],
        )
        .await?;
        Ok(())
    }

    async fn delete(&self, sid: &str) -> Result<()> {
        self.exec(
            "DELETE FROM loco_sessions WHERE id_hash = ?",
            vec![sid.into()],
        )
        .await?;
        Ok(())
    }

    async fn revoke_user(&self, user: &str) -> Result<u64> {
        let sessions = self
            .exec(
                "DELETE FROM loco_sessions WHERE user_key = ?",
                vec![user.into()],
            )
            .await?;
        let remember = self
            .exec(
                "DELETE FROM loco_remember_tokens WHERE user_key = ?",
                vec![user.into()],
            )
            .await?;
        Ok(sessions + remember)
    }

    async fn remember_get(&self, series: &str) -> Result<Option<Remember>> {
        let row = self
            .db
            .query_one_raw(self.statement(
                "SELECT user_key, token_hash, prev_hash, rotated_at, expires_at FROM \
                 loco_remember_tokens WHERE series = ?",
                vec![series.into()],
            ))
            .await?;
        let Some(row) = row else { return Ok(None) };
        Ok(Some(Remember {
            user: row.try_get("", "user_key")?,
            token_hash: row.try_get("", "token_hash")?,
            prev_hash: row.try_get("", "prev_hash")?,
            rotated_at: row.try_get("", "rotated_at")?,
            expires_at: row.try_get("", "expires_at")?,
        }))
    }

    async fn remember_put(&self, series: &str, r: &Remember) -> Result<()> {
        self.exec(
            "INSERT INTO loco_remember_tokens (series, user_key, token_hash, prev_hash, \
             rotated_at, expires_at) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT (series) DO UPDATE SET \
             user_key = excluded.user_key, token_hash = excluded.token_hash, prev_hash = \
             excluded.prev_hash, rotated_at = excluded.rotated_at, expires_at = \
             excluded.expires_at",
            vec![
                series.into(),
                r.user.clone().into(),
                r.token_hash.clone().into(),
                r.prev_hash.clone().into(),
                r.rotated_at.into(),
                r.expires_at.into(),
            ],
        )
        .await?;
        Ok(())
    }

    async fn remember_delete(&self, series: &str) -> Result<()> {
        self.exec(
            "DELETE FROM loco_remember_tokens WHERE series = ?",
            vec![series.into()],
        )
        .await?;
        Ok(())
    }

    async fn purge_expired(&self, now: i64) -> Result<u64> {
        let mut deleted = 0;
        for (table, key) in [
            ("loco_sessions", "id_hash"),
            ("loco_remember_tokens", "series"),
        ] {
            deleted += self
                .exec(
                    &format!(
                        "DELETE FROM {table} WHERE {key} IN (SELECT {key} FROM {table} WHERE \
                         expires_at < ? LIMIT {PURGE_BATCH})"
                    ),
                    vec![now.into()],
                )
                .await?;
        }
        Ok(deleted)
    }
}
