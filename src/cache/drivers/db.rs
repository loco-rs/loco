//! # Database Cache Driver
//!
//! A cache that lives in the application's own database (the `Solid Cache`
//! idea from Rails 8): no Redis, no extra service, and the cache survives a
//! restart. Works with Postgres and `SQLite`.
//!
//! Entries are rows of the `loco_cache` table, which the driver creates on
//! first use (`CREATE TABLE IF NOT EXISTS`, like the SQL job queues do). A row
//! past its `expires_at` counts as missing, and every `sweep_every` writes the
//! driver deletes a batch of expired rows.
use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, FromQueryResult, Statement, Value,
};

use super::CacheDriver;
use crate::cache::{CacheError, CacheResult};
use crate::config::DatabaseCacheConfig;

/// Name of the table holding the cache entries.
pub const TABLE: &str = "loco_cache";

/// Creates a new instance of the database cache driver on top of `db`.
#[must_use]
pub fn new(config: &DatabaseCacheConfig, db: &DatabaseConnection) -> crate::cache::Cache {
    crate::cache::Cache::new(Box::new(Database::new(
        db.clone(),
        config.sweep_every,
        config.sweep_batch,
    )))
}

/// Represents the database cache driver.
#[derive(Debug)]
pub struct Database {
    db: DatabaseConnection,
    ready: AtomicBool,
    writes: AtomicU64,
    sweep_every: u64,
    sweep_batch: u64,
}

#[derive(FromQueryResult)]
struct Row {
    value: Vec<u8>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

impl Database {
    /// Constructs the driver. `sweep_every` is the number of writes between
    /// sweeps of expired rows (`0` never sweeps); each sweep deletes at most
    /// `sweep_batch` rows.
    #[must_use]
    pub fn new(db: DatabaseConnection, sweep_every: u64, sweep_batch: u64) -> Self {
        Self {
            db,
            ready: AtomicBool::new(false),
            writes: AtomicU64::new(0),
            sweep_every,
            sweep_batch,
        }
    }

    fn backend(&self) -> DatabaseBackend {
        self.db.get_database_backend()
    }

    /// SQL with `?` placeholders, rewritten to `$n` for Postgres.
    fn stmt(&self, sql: &str, values: Vec<Value>) -> Statement {
        let backend = self.backend();
        let sql = if backend == DatabaseBackend::Postgres {
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
        Statement::from_sql_and_values(backend, sql, values)
    }

    async fn exec(&self, sql: &str, values: Vec<Value>) -> CacheResult<u64> {
        self.ensure_table().await?;
        Ok(self
            .db
            .execute_raw(self.stmt(sql, values))
            .await?
            .rows_affected())
    }

    /// Creates the table if it is not there. Runs once per driver and again
    /// only if the first attempt failed; the statements are idempotent, so a
    /// race between two first calls is harmless.
    ///
    /// It happens on first use rather than at boot so that a
    /// `dangerously_recreate` reset, which drops every table after the
    /// context is built, cannot leave the cache without its table.
    async fn ensure_table(&self) -> CacheResult<()> {
        if self.ready.load(Ordering::Acquire) {
            return Ok(());
        }
        let blob = if self.backend() == DatabaseBackend::Postgres {
            "BYTEA"
        } else {
            "BLOB"
        };
        self.db
            .execute_raw(Statement::from_string(
                self.backend(),
                format!(
                    "CREATE TABLE IF NOT EXISTS {TABLE} (\"key\" TEXT PRIMARY KEY NOT NULL, \
                     value {blob} NOT NULL, expires_at BIGINT, created_at BIGINT NOT NULL)"
                ),
            ))
            .await?;
        self.db
            .execute_raw(Statement::from_string(
                self.backend(),
                format!(
                    "CREATE INDEX IF NOT EXISTS idx_{TABLE}_expires_at ON {TABLE} (expires_at)"
                ),
            ))
            .await?;
        self.ready.store(true, Ordering::Release);
        Ok(())
    }

    async fn put(&self, key: &str, value: &str, expires_at: Option<i64>) -> CacheResult<()> {
        self.exec(
            &format!(
                "INSERT INTO {TABLE} (\"key\", value, expires_at, created_at) VALUES (?, ?, ?, ?) \
                 ON CONFLICT (\"key\") DO UPDATE SET value = excluded.value, \
                 expires_at = excluded.expires_at, created_at = excluded.created_at"
            ),
            vec![
                key.into(),
                value.as_bytes().to_vec().into(),
                expires_at.into(),
                now_ms().into(),
            ],
        )
        .await?;
        self.sweep_if_due().await
    }

    async fn sweep_if_due(&self) -> CacheResult<()> {
        if self.sweep_every == 0 {
            return Ok(());
        }
        let writes = self.writes.fetch_add(1, Ordering::Relaxed) + 1;
        if writes.is_multiple_of(self.sweep_every) {
            self.sweep().await?;
        }
        Ok(())
    }

    /// Deletes up to `sweep_batch` expired rows and returns how many went.
    ///
    /// # Errors
    ///
    /// Returns a [`CacheError`] when the database fails.
    pub async fn sweep(&self) -> CacheResult<u64> {
        let limit = i64::try_from(self.sweep_batch).unwrap_or(i64::MAX);
        self.exec(
            &format!(
                "DELETE FROM {TABLE} WHERE \"key\" IN (SELECT \"key\" FROM {TABLE} \
                 WHERE expires_at IS NOT NULL AND expires_at <= ? LIMIT ?)"
            ),
            vec![now_ms().into(), limit.into()],
        )
        .await
    }
}

#[async_trait]
impl CacheDriver for Database {
    async fn ping(&self) -> CacheResult<()> {
        self.ensure_table().await?;
        self.db.ping().await?;
        Ok(())
    }

    async fn contains_key(&self, key: &str) -> CacheResult<bool> {
        Ok(self.get(key).await?.is_some())
    }

    async fn get(&self, key: &str) -> CacheResult<Option<String>> {
        self.ensure_table().await?;
        let row = Row::find_by_statement(self.stmt(
            &format!(
                "SELECT value FROM {TABLE} WHERE \"key\" = ? \
                 AND (expires_at IS NULL OR expires_at > ?)"
            ),
            vec![key.into(), now_ms().into()],
        ))
        .one(&self.db)
        .await?;
        row.map(|row| {
            String::from_utf8(row.value).map_err(|e| CacheError::Deserialization(e.to_string()))
        })
        .transpose()
    }

    async fn insert(&self, key: &str, value: &str) -> CacheResult<()> {
        self.put(key, value, None).await
    }

    async fn insert_with_expiry(
        &self,
        key: &str,
        value: &str,
        duration: Duration,
    ) -> CacheResult<()> {
        let ttl = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
        self.put(key, value, Some(now_ms().saturating_add(ttl)))
            .await
    }

    async fn remove(&self, key: &str) -> CacheResult<()> {
        self.exec(
            &format!("DELETE FROM {TABLE} WHERE \"key\" = ?"),
            vec![key.into()],
        )
        .await?;
        Ok(())
    }

    async fn clear(&self) -> CacheResult<()> {
        self.exec(&format!("DELETE FROM {TABLE}"), vec![]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg::{config::get_sqlite_test_config, postgres::setup_postgres_container};

    async fn sqlite() -> (Database, tree_fs::Tree) {
        let (config, tree) = get_sqlite_test_config("cache");
        let db = crate::db::connect(&config).await.unwrap();
        (Database::new(db, 100, 1000), tree)
    }

    async fn count(cache: &Database) -> i64 {
        let row = cache
            .db
            .query_one_raw(Statement::from_string(
                cache.backend(),
                format!("SELECT COUNT(*) AS n FROM {TABLE}"),
            ))
            .await
            .unwrap()
            .unwrap();
        row.try_get::<i64>("", "n").unwrap()
    }

    /// The same behaviors the in-memory and Redis drivers are tested for.
    async fn behaves_like_a_cache(cache: &Database) {
        cache.ping().await.unwrap();

        assert!(!cache.contains_key("key").await.unwrap());
        assert_eq!(cache.get("key").await.unwrap(), None);
        cache.insert("key", "loco").await.unwrap();
        assert!(cache.contains_key("key").await.unwrap());
        assert_eq!(cache.get("key").await.unwrap().as_deref(), Some("loco"));
        cache.insert("key", "again").await.unwrap();
        assert_eq!(cache.get("key").await.unwrap().as_deref(), Some("again"));
        cache.insert("uni", "héllo ✓").await.unwrap();
        assert_eq!(cache.get("uni").await.unwrap().as_deref(), Some("héllo ✓"));

        cache.remove("key").await.unwrap();
        assert!(!cache.contains_key("key").await.unwrap());
        cache.remove("never-existed").await.unwrap();

        cache
            .insert_with_expiry("short", "a", Duration::from_millis(50))
            .await
            .unwrap();
        cache
            .insert_with_expiry("long", "b", Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(cache.get("short").await.unwrap().as_deref(), Some("a"));
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(cache.get("short").await.unwrap(), None);
        assert!(!cache.contains_key("short").await.unwrap());
        assert_eq!(cache.get("long").await.unwrap().as_deref(), Some("b"));
        // A plain insert over an expiring entry keeps it for good.
        cache
            .insert_with_expiry("temp", "x", Duration::from_millis(50))
            .await
            .unwrap();
        cache.insert("temp", "y").await.unwrap();
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(cache.get("temp").await.unwrap().as_deref(), Some("y"));

        cache.clear().await.unwrap();
        for key in ["uni", "long", "temp"] {
            assert!(!cache.contains_key(key).await.unwrap());
        }
        assert_eq!(count(cache).await, 0);
    }

    async fn sweeps_expired_rows(db: DatabaseConnection) {
        // Sweep after every write, at most two rows at a time.
        let cache = Database::new(db, 1, 2);
        cache.clear().await.unwrap();
        for i in 0..5 {
            cache
                .insert_with_expiry(&format!("old{i}"), "v", Duration::ZERO)
                .await
                .unwrap();
        }
        assert!(count(&cache).await < 5, "writes sweep expired rows");
        cache.insert("keep", "v").await.unwrap();
        cache
            .insert_with_expiry("later", "v", Duration::from_secs(60))
            .await
            .unwrap();
        while cache.sweep().await.unwrap() > 0 {}
        assert_eq!(count(&cache).await, 2, "sweep leaves live rows alone");
        assert!(cache.contains_key("keep").await.unwrap());
        assert!(cache.contains_key("later").await.unwrap());
    }

    #[tokio::test]
    async fn sqlite_behaves_like_a_cache() {
        let (cache, _tree) = sqlite().await;
        behaves_like_a_cache(&cache).await;
    }

    #[tokio::test]
    async fn sqlite_sweeps_expired_rows() {
        let (cache, _tree) = sqlite().await;
        sweeps_expired_rows(cache.db.clone()).await;
    }

    #[tokio::test]
    async fn sqlite_creates_the_table_on_first_use_and_again_after_a_reset() {
        let (cache, _tree) = sqlite().await;
        cache.insert("a", "1").await.unwrap();
        cache
            .db
            .execute_unprepared(&format!("DROP TABLE {TABLE}"))
            .await
            .unwrap();
        // A fresh driver (a restart) finds no table and makes one.
        let fresh = Database::new(cache.db.clone(), 100, 1000);
        assert_eq!(fresh.get("a").await.unwrap(), None);
        fresh.insert("a", "2").await.unwrap();
        assert_eq!(fresh.get("a").await.unwrap().as_deref(), Some("2"));
    }

    #[tokio::test]
    async fn sqlite_cache_survives_a_restart() {
        let (config, _tree) = get_sqlite_test_config("restart");
        let first = Database::new(crate::db::connect(&config).await.unwrap(), 100, 1000);
        first.insert("k", "v").await.unwrap();
        drop(first);
        let second = Database::new(crate::db::connect(&config).await.unwrap(), 100, 1000);
        assert_eq!(second.get("k").await.unwrap().as_deref(), Some("v"));
    }

    #[tokio::test]
    async fn postgres_behaves_like_a_cache() {
        let (url, _container) = setup_postgres_container().await;
        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = url;
        let db = crate::db::connect(&config).await.unwrap();
        behaves_like_a_cache(&Database::new(db.clone(), 100, 1000)).await;
        sweeps_expired_rows(db).await;
    }
}
