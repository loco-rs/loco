//! The `database` store: `loco_sessions` and `loco_remember_tokens`, created
//! idempotently on first use (the way the Postgres and `SQLite` queues create
//! theirs), on Postgres and `SQLite`.

use std::fmt::Write as _;

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait, Value};

use super::{Backend, Op, Record, Remember};
use crate::{Error, Result};

/// Rows deleted per purge.
const PURGE_BATCH: u32 = 500;

/// Serialises concurrent first-use DDL on Postgres (the bytes of `loco_ses`).
const DDL_LOCK: i64 = 0x6c6f_636f_5f73_6573;

#[derive(Debug)]
pub(super) struct Database {
    db: DatabaseConnection,
    backend: DbBackend,
}

/// Statements are written with `?`; Postgres wants `$1, $2, ...`.
fn statement(backend: DbBackend, sql: &str, values: Vec<Value>) -> Statement {
    let sql = if matches!(backend, DbBackend::Postgres) {
        let mut n = 0;
        let mut out = String::with_capacity(sql.len() + 8);
        for c in sql.chars() {
            if c == '?' {
                n += 1;
                let _ = write!(out, "${n}");
            } else {
                out.push(c);
            }
        }
        out
    } else {
        sql.to_string()
    };
    Statement::from_sql_and_values(backend, sql, values)
}

async fn exec<C: ConnectionTrait>(
    conn: &C,
    backend: DbBackend,
    sql: &str,
    values: Vec<Value>,
) -> Result<u64> {
    Ok(conn
        .execute_raw(statement(backend, sql, values))
        .await?
        .rows_affected())
}

fn json(record: &Record) -> Result<String> {
    Ok(serde_json::to_string(&record.data)?)
}

impl Database {
    pub(super) async fn init(db: DatabaseConnection) -> Result<Self> {
        let backend = db.get_database_backend();
        if matches!(backend, DbBackend::MySql) {
            return Err(Error::Message(
                "auth.session.store `database` supports Postgres and SQLite".to_string(),
            ));
        }
        // One transaction, and on Postgres one at a time: two processes
        // booting together would otherwise race on `CREATE TABLE IF NOT
        // EXISTS` (it is not atomic there) and one would fail.
        let txn = db.begin().await?;
        if matches!(backend, DbBackend::Postgres) {
            exec(
                &txn,
                backend,
                &format!("SELECT pg_advisory_xact_lock({DDL_LOCK})"),
                vec![],
            )
            .await?;
        }
        for ddl in [
            "CREATE TABLE IF NOT EXISTS loco_sessions (id_hash VARCHAR(64) PRIMARY KEY, user_key \
             TEXT, data TEXT NOT NULL, expires_at BIGINT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS loco_sessions_user_key_idx ON loco_sessions (user_key)",
            "CREATE INDEX IF NOT EXISTS loco_sessions_expires_at_idx ON loco_sessions \
             (expires_at)",
            "CREATE TABLE IF NOT EXISTS loco_remember_tokens (series VARCHAR(64) PRIMARY KEY, \
             user_key TEXT NOT NULL, generation BIGINT NOT NULL, rotated_at BIGINT NOT \
             NULL, expires_at BIGINT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS loco_remember_tokens_user_key_idx ON \
             loco_remember_tokens (user_key)",
        ] {
            exec(&txn, backend, ddl, vec![]).await?;
        }
        txn.commit().await?;
        Ok(Self { db, backend })
    }

    async fn run<C: ConnectionTrait>(&self, conn: &C, op: &Op) -> Result<()> {
        let b = self.backend;
        match op {
            Op::Insert { id, record } => {
                exec(
                    conn,
                    b,
                    "INSERT INTO loco_sessions (id_hash, user_key, data, expires_at) VALUES (?, \
                     ?, ?, ?)",
                    vec![
                        id.as_str().into(),
                        record.user.clone().into(),
                        json(record)?.into(),
                        record.expires_at.into(),
                    ],
                )
                .await?;
            }
            Op::Update { id, record } => {
                // Only a row that is still there: a session ended while this
                // request was in flight stays ended.
                exec(
                    conn,
                    b,
                    "UPDATE loco_sessions SET user_key = ?, data = ?, expires_at = ? WHERE \
                     id_hash = ?",
                    vec![
                        record.user.clone().into(),
                        json(record)?.into(),
                        record.expires_at.into(),
                        id.as_str().into(),
                    ],
                )
                .await?;
            }
            Op::Delete { id } => {
                exec(
                    conn,
                    b,
                    "DELETE FROM loco_sessions WHERE id_hash = ?",
                    vec![id.as_str().into()],
                )
                .await?;
            }
            Op::RememberInsert { series, remember } => {
                exec(
                    conn,
                    b,
                    "INSERT INTO loco_remember_tokens (series, user_key, generation, rotated_at, \
                     expires_at) VALUES (?, ?, ?, ?, ?)",
                    vec![
                        series.as_str().into(),
                        remember.user.clone().into(),
                        remember.generation.into(),
                        remember.rotated_at.into(),
                        remember.expires_at.into(),
                    ],
                )
                .await?;
            }
            Op::RememberDelete { series } => {
                exec(
                    conn,
                    b,
                    "DELETE FROM loco_remember_tokens WHERE series = ?",
                    vec![series.as_str().into()],
                )
                .await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Backend for Database {
    async fn load(&self, sid: &str) -> Result<Option<Record>> {
        let row = self
            .db
            .query_one_raw(statement(
                self.backend,
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

    async fn insert(&self, sid: &str, record: &Record) -> Result<()> {
        self.run(
            &self.db,
            &Op::Insert {
                id: sid.to_string(),
                record: record.clone(),
            },
        )
        .await
    }

    async fn insert_restored(&self, sid: &str, record: &Record, series: &str) -> Result<bool> {
        // One statement: the row exists only if the series still did when it
        // ran, so a revoke cannot slip in between a check and the insert.
        Ok(exec(
            &self.db,
            self.backend,
            "INSERT INTO loco_sessions (id_hash, user_key, data, expires_at) SELECT ?, ?, ?, ? \
             WHERE EXISTS (SELECT 1 FROM loco_remember_tokens WHERE series = ?)",
            vec![
                sid.into(),
                record.user.clone().into(),
                json(record)?.into(),
                record.expires_at.into(),
                series.into(),
            ],
        )
        .await?
            > 0)
    }

    async fn update(&self, sid: &str, record: &Record) -> Result<bool> {
        Ok(exec(
            &self.db,
            self.backend,
            "UPDATE loco_sessions SET user_key = ?, data = ?, expires_at = ? WHERE id_hash = ?",
            vec![
                record.user.clone().into(),
                json(record)?.into(),
                record.expires_at.into(),
                sid.into(),
            ],
        )
        .await?
            > 0)
    }

    async fn delete(&self, sid: &str) -> Result<()> {
        self.run(
            &self.db,
            &Op::Delete {
                id: sid.to_string(),
            },
        )
        .await
    }

    async fn revoke_user(&self, user: &str) -> Result<u64> {
        let txn = self.db.begin().await?;
        let sessions = exec(
            &txn,
            self.backend,
            "DELETE FROM loco_sessions WHERE user_key = ?",
            vec![user.into()],
        )
        .await?;
        let remember = exec(
            &txn,
            self.backend,
            "DELETE FROM loco_remember_tokens WHERE user_key = ?",
            vec![user.into()],
        )
        .await?;
        txn.commit().await?;
        Ok(sessions + remember)
    }

    async fn remember_get(&self, series: &str) -> Result<Option<Remember>> {
        let row = self
            .db
            .query_one_raw(statement(
                self.backend,
                "SELECT user_key, generation, rotated_at, expires_at FROM loco_remember_tokens \
                 WHERE series = ?",
                vec![series.into()],
            ))
            .await?;
        let Some(row) = row else { return Ok(None) };
        Ok(Some(Remember {
            user: row.try_get("", "user_key")?,
            generation: row.try_get("", "generation")?,
            rotated_at: row.try_get("", "rotated_at")?,
            expires_at: row.try_get("", "expires_at")?,
        }))
    }

    async fn remember_insert(&self, series: &str, remember: &Remember) -> Result<()> {
        self.run(
            &self.db,
            &Op::RememberInsert {
                series: series.to_string(),
                remember: remember.clone(),
            },
        )
        .await
    }

    async fn remember_cas(
        &self,
        series: &str,
        expected_generation: i64,
        r: &Remember,
    ) -> Result<bool> {
        Ok(exec(
            &self.db,
            self.backend,
            "UPDATE loco_remember_tokens SET generation = ?, rotated_at = ?, expires_at = ? \
             WHERE series = ? AND generation = ?",
            vec![
                r.generation.into(),
                r.rotated_at.into(),
                r.expires_at.into(),
                series.into(),
                expected_generation.into(),
            ],
        )
        .await?
            > 0)
    }

    async fn remember_delete(&self, series: &str) -> Result<()> {
        self.run(
            &self.db,
            &Op::RememberDelete {
                series: series.to_string(),
            },
        )
        .await
    }

    async fn purge_expired(&self, now: i64) -> Result<u64> {
        let mut deleted = 0;
        for (table, key) in [
            ("loco_sessions", "id_hash"),
            ("loco_remember_tokens", "series"),
        ] {
            deleted += exec(
                &self.db,
                self.backend,
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

    /// All or nothing: a sign-out never leaves the remember series alive
    /// while the session is gone, or the other way round.
    async fn apply(&self, ops: &[Op]) -> Result<()> {
        let txn = self.db.begin().await?;
        for op in ops {
            self.run(&txn, op).await?;
        }
        txn.commit().await?;
        Ok(())
    }
}
