/// `SQLite` based background job queue provider
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
pub use sqlx::SqlitePool;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
    ConnectOptions, QueryBuilder, Row,
};
use tokio::{task::JoinHandle, time::sleep};
use tracing::{debug, error, trace};
use ulid::Ulid;

use super::{BackgroundWorker, JobStatus, Queue};
use crate::{config::SqliteQueueConfig, Error, Result};
type JobId = String;
type JobData = JsonValue;

type JobHandler = Box<
    dyn Fn(
            JobId,
            JobData,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<(), crate::Error>> + Send>>
        + Send
        + Sync,
>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Job {
    pub id: JobId,
    pub name: String,
    #[serde(rename = "task_data")]
    pub data: JobData,
    pub status: JobStatus,
    pub run_at: DateTime<Utc>,
    pub interval: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

pub struct JobRegistry {
    handlers: Arc<HashMap<String, JobHandler>>,
}

impl JobRegistry {
    /// Creates a new `JobRegistry`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(HashMap::new()),
        }
    }

    /// Registers a job handler with the provided name.
    /// # Errors
    /// Fails if cannot register worker
    pub fn register_worker<Args, W>(&mut self, name: String, worker: W) -> Result<()>
    where
        Args: Send + Serialize + Sync + 'static,
        W: BackgroundWorker<Args> + 'static,
        for<'de> Args: Deserialize<'de>,
    {
        let worker = Arc::new(worker);
        let wrapped_handler = move |_job_id: String, job_data: JobData| {
            let w = worker.clone();

            Box::pin(async move {
                let args = serde_json::from_value::<Args>(job_data);
                match args {
                    Ok(args) => w.perform(args).await,
                    Err(err) => Err(err.into()),
                }
            }) as Pin<Box<dyn Future<Output = Result<(), crate::Error>> + Send>>
        };

        Arc::get_mut(&mut self.handlers)
            .ok_or_else(|| Error::string("cannot register worker"))?
            .insert(name, Box::new(wrapped_handler));
        Ok(())
    }

    /// Returns a reference to the job handlers.
    #[must_use]
    pub fn handlers(&self) -> &Arc<HashMap<String, JobHandler>> {
        &self.handlers
    }

    /// Runs the job handlers with the provided number of workers.
    #[must_use]
    pub fn run(&self, pool: &SqlitePool, opts: &RunOpts) -> Vec<JoinHandle<()>> {
        let mut jobs = Vec::new();

        let interval = opts.poll_interval_sec;
        for idx in 0..opts.num_workers {
            let handlers = self.handlers.clone();

            let pool = pool.clone();
            let job: JoinHandle<()> = tokio::spawn(async move {
                loop {
                    trace!(
                        pool_conns = pool.num_idle(),
                        worker_num = idx,
                        "sqlite workers stats"
                    );
                    let job_opt = match dequeue(&pool).await {
                        Ok(t) => t,
                        Err(err) => {
                            error!(err = err.to_string(), "cannot fetch from queue");
                            None
                        }
                    };

                    if let Some(job) = job_opt {
                        debug!(job_id = job.id, name = job.name, "working on job");
                        if let Some(handler) = handlers.get(&job.name) {
                            match handler(job.id.clone(), job.data.clone()).await {
                                Ok(()) => {
                                    if let Err(err) =
                                        complete_job(&pool, &job.id, job.interval).await
                                    {
                                        error!(
                                            err = err.to_string(),
                                            job = ?job,
                                            "cannot complete job"
                                        );
                                    }
                                }
                                Err(err) => {
                                    if let Err(err) = fail_job(&pool, &job.id, &err).await {
                                        error!(
                                            err = err.to_string(),
                                            job = ?job,
                                            "cannot fail job"
                                        );
                                    }
                                }
                            }
                        } else {
                            error!(job_name = job.name, "no handler found for job");
                        }
                    } else {
                        sleep(Duration::from_secs(interval.into())).await;
                    }
                }
            });

            jobs.push(job);
        }

        jobs
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

async fn connect(cfg: &SqliteQueueConfig) -> Result<SqlitePool> {
    let mut conn_opts: SqliteConnectOptions = cfg.uri.parse()?;
    if !cfg.enable_logging {
        conn_opts = conn_opts.disable_statement_logging();
    }
    let pool = SqlitePoolOptions::new()
        .min_connections(cfg.min_connections)
        .max_connections(cfg.max_connections)
        .idle_timeout(Duration::from_millis(cfg.idle_timeout))
        .acquire_timeout(Duration::from_millis(cfg.connect_timeout))
        .connect_with(conn_opts)
        .await?;
    Ok(pool)
}

/// Initialize job tables
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn initialize_database(pool: &SqlitePool) -> Result<()> {
    debug!("sqlite worker: initialize database");
    sqlx::query(
        &format!(r"
            CREATE TABLE IF NOT EXISTS sqlt_loco_queue (
                id TEXT NOT NULL,
                name TEXT NOT NULL,
                task_data JSON NOT NULL,
                status TEXT NOT NULL DEFAULT '{}',
                run_at TIMESTAMP NOT NULL,
                interval INTEGER,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS sqlt_loco_queue_lock (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                is_locked BOOLEAN NOT NULL DEFAULT FALSE,
                locked_at TIMESTAMP NULL
            );

            INSERT OR IGNORE INTO sqlt_loco_queue_lock (id, is_locked) VALUES (1, FALSE);

            CREATE INDEX IF NOT EXISTS idx_sqlt_queue_status_run_at ON sqlt_loco_queue(status, run_at);
            ", JobStatus::Queued),
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Add a job
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn enqueue(
    pool: &SqlitePool,
    name: &str,
    data: JobData,
    run_at: DateTime<Utc>,
    interval: Option<Duration>,
) -> Result<JobId> {
    let data = serde_json::to_value(data)?;

    #[allow(clippy::cast_possible_truncation)]
    let interval_ms: Option<i64> = interval.map(|i| i.as_millis() as i64);

    let id = Ulid::new().to_string();
    sqlx::query(
        "INSERT INTO sqlt_loco_queue (id, task_data, name, run_at, interval) VALUES ($1, $2, $3, \
         DATETIME($4), $5)",
    )
    .bind(id.clone())
    .bind(data)
    .bind(name)
    .bind(run_at)
    .bind(interval_ms)
    .execute(pool)
    .await?;
    Ok(id)
}

async fn dequeue(client: &SqlitePool) -> Result<Option<Job>> {
    let mut tx = client.begin().await?;

    let acquired_write_lock = sqlx::query(
        "UPDATE sqlt_loco_queue_lock SET
            is_locked = TRUE,
            locked_at = CURRENT_TIMESTAMP
        WHERE id = 1 AND is_locked = FALSE",
    )
    .execute(&mut *tx)
    .await?;

    // Couldn't aquire the write lock
    if acquired_write_lock.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(None);
    }

    let row = sqlx::query(
        "SELECT id, name, task_data, status, run_at, interval
        FROM sqlt_loco_queue
        WHERE
            status = ? AND
            run_at <= CURRENT_TIMESTAMP
        ORDER BY run_at LIMIT 1",
    )
    .bind(JobStatus::Queued.to_string())
    .map(|row: SqliteRow| to_job(&row).ok())
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    if let Some(job) = row {
        sqlx::query(
            "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(JobStatus::Processing.to_string())
        .bind(&job.id)
        .execute(&mut *tx)
        .await?;

        // Release the write lock
        sqlx::query(
            "UPDATE sqlt_loco_queue_lock 
              SET is_locked = FALSE,
                  locked_at = NULL
              WHERE id = 1",
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(job))
    } else {
        // Release the write lock, no job found
        sqlx::query(
            "UPDATE sqlt_loco_queue_lock 
              SET is_locked = FALSE,
                  locked_at = NULL
              WHERE id = 1",
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(None)
    }
}

async fn complete_job(pool: &SqlitePool, id: &JobId, interval_ms: Option<i64>) -> Result<()> {
    if let Some(interval_ms) = interval_ms {
        let next_run_at = Utc::now() + chrono::Duration::milliseconds(interval_ms);
        sqlx::query(
            "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP, run_at = \
             DATETIME($2) WHERE id = $3",
        )
        .bind(JobStatus::Queued.to_string())
        .bind(next_run_at)
        .bind(id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(JobStatus::Completed.to_string())
        .bind(id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn fail_job(pool: &SqlitePool, id: &JobId, error: &crate::Error) -> Result<()> {
    let msg = error.to_string();
    error!(err = msg, "failed job");
    let error_json = serde_json::json!({ "error": msg });
    sqlx::query(
        "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP, task_data = \
         json_patch(task_data, $2) WHERE id = $3",
    )
    .bind(JobStatus::Failed.to_string())
    .bind(error_json)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Cancels jobs in the `sqlt_loco_queue` table by their name.
///
/// This function updates the status of all jobs with the given `name` and a status of
/// [`JobStatus::Queued`] to [`JobStatus::Cancelled`]. The update also sets the `updated_at` timestamp to the
/// current time.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn cancel_jobs_by_name(pool: &SqlitePool, name: &str) -> Result<()> {
    sqlx::query(
        "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE name = $2 \
         AND status = $3",
    )
    .bind(JobStatus::Cancelled.to_string())
    .bind(name)
    .bind(JobStatus::Queued.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Clear all jobs
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear(pool: &SqlitePool) -> Result<()> {
    // Clear all rows in the relevant tables
    sqlx::query(
        "
        DELETE FROM sqlt_loco_queue;
        DELETE FROM sqlt_loco_queue_lock;
        ",
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Deletes jobs from the `sqlt_loco_queue` table based on their status.
///
/// This function removes all jobs with a status that matches any of the statuses provided
/// in the `status` argument. The statuses are checked against the `status` column in the
/// database, and any matching rows are deleted.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear_by_status(pool: &SqlitePool, status: Vec<JobStatus>) -> Result<()> {
    let status_in = status
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<String>>()
        .join(",");

    sqlx::query(&format!(
        "DELETE FROM sqlt_loco_queue WHERE status IN ({status_in})"
    ))
    .execute(pool)
    .await?;

    Ok(())
}

/// Requeues jobs from [`JobStatus::Processing`] to [`JobStatus::Queued`].
///
/// This function updates the status of all jobs that are currently in the [`JobStatus::Processing`] state
/// to the [`JobStatus::Queued`] state, provided they have been updated more than the specified age (`age_minutes`).
/// The jobs that meet the criteria will have their `updated_at` timestamp set to the current time.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn requeue(pool: &SqlitePool, age_minutes: &i64) -> Result<()> {
    let query = format!(
        "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE status = $2 AND updated_at <= DATETIME('now', '-{age_minutes} minute')"
    );

    sqlx::query(&query)
        .bind(JobStatus::Queued.to_string())
        .bind(JobStatus::Processing.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

/// Deletes jobs from the `sqlt_loco_queue` table that are older than a specified number of days.
///
/// This function removes jobs that have a `created_at` timestamp older than the provided
/// number of days. Additionally, if a `status` is provided, only jobs with a status matching
/// one of the provided values will be deleted.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear_jobs_older_than(
    pool: &SqlitePool,
    age_days: i64,
    status: Option<&Vec<JobStatus>>,
) -> Result<()> {
    let cutoff_date = Utc::now() - chrono::Duration::days(age_days);
    let threshold_date = cutoff_date.format("%+").to_string();

    let mut query_builder =
        QueryBuilder::<sqlx::Sqlite>::new("DELETE FROM sqlt_loco_queue WHERE created_at <= ");
    query_builder.push_bind(threshold_date);

    if let Some(status_list) = status {
        if !status_list.is_empty() {
            let status_in = status_list
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<String>>()
                .join(",");

            query_builder.push(format!(" AND status IN ({status_in})"));
        }
    }

    query_builder.build().execute(pool).await?;
    Ok(())
}

/// Ping system
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn ping(pool: &SqlitePool) -> Result<()> {
    sqlx::query("SELECT id from sqlt_loco_queue LIMIT 1")
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug)]
pub struct RunOpts {
    pub num_workers: u32,
    pub poll_interval_sec: u32,
}

/// Create this provider
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn create_provider(qcfg: &SqliteQueueConfig) -> Result<Queue> {
    let pool = connect(qcfg).await.map_err(Box::from)?;
    let registry = JobRegistry::new();
    Ok(Queue::Sqlite(
        pool,
        Arc::new(tokio::sync::Mutex::new(registry)),
        RunOpts {
            num_workers: qcfg.num_workers,
            poll_interval_sec: qcfg.poll_interval_sec,
        },
    ))
}

/// Retrieves a list of jobs from the `sqlt_loco_queue` table in the database.
///
/// This function queries the database for jobs, optionally filtering by their
/// `status`. If a status is provided, only jobs with statuses included in the
/// provided list will be fetched. If no status is provided, all jobs will be
/// returned.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn get_jobs(
    pool: &SqlitePool,
    status: Option<&Vec<JobStatus>>,
    age_days: Option<i64>,
) -> Result<Vec<Job>> {
    let mut query = String::from("SELECT * FROM sqlt_loco_queue WHERE 1 = 1 ");

    if let Some(status) = status {
        let status_in = status
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<String>>()
            .join(",");
        query.push_str(&format!("AND status IN ({status_in}) "));
    }

    if let Some(age_days) = age_days {
        let cutoff_date = Utc::now() - chrono::Duration::days(age_days);
        let threshold_date = cutoff_date.format("%+").to_string();
        query.push_str(&format!("AND created_at <= '{threshold_date}' "));
    }

    let rows = sqlx::query(&query).fetch_all(pool).await?;
    Ok(rows.iter().filter_map(|row| to_job(row).ok()).collect())
}

/// Converts a row from the database into a [`Job`] object.
///
/// This function takes a row from the `SQLite` database and manually extracts the necessary
/// fields to populate a [`Job`] object.
///
/// **Note:** This function manually extracts values from the database row instead of using
/// the `FromRow` trait, which would require enabling the 'macros' feature in the dependencies.
/// The decision to avoid `FromRow` is made to keep the build smaller and faster, as the 'macros'
/// feature is unnecessary in the current dependency tree.
fn to_job(row: &SqliteRow) -> Result<Job> {
    Ok(Job {
        id: row.get("id"),
        name: row.get("name"),
        data: row.get("task_data"),
        status: row.get::<String, _>("status").parse().map_err(|err| {
            let status: String = row.get("status");
            tracing::error!(status, err, "job status is unsupported");
            Error::string("invalid job status")
        })?,
        run_at: row.get("run_at"),
        interval: row.get("interval"),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use chrono::{NaiveDate, NaiveTime, TimeZone};
    use insta::{assert_debug_snapshot, with_settings};
    use sqlx::{query_as, FromRow, Pool, Sqlite};

    use super::*;
    use crate::tests_cfg;

    #[derive(Debug, Serialize, FromRow)]
    pub struct TableInfo {
        cid: i32,
        name: String,
        #[sqlx(rename = "type")]
        _type: String,
        notnull: bool,
        dflt_value: Option<String>,
        pk: bool,
    }

    #[derive(Debug, Serialize, FromRow)]
    struct JobQueueLock {
        id: i32,
        is_locked: bool,
        locked_at: Option<DateTime<Utc>>,
    }

    fn reduction() -> &'static [(&'static str, &'static str)] {
        &[
            ("[A-Z0-9]{26}", "<REDACTED>"),
            (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", "<REDACTED>"),
        ]
    }

    async fn init(db_path: &Path) -> Pool<Sqlite> {
        let qcfg = SqliteQueueConfig {
            uri: format!(
                "sqlite://{}?mode=rwc",
                db_path.join("sample.sqlite").display()
            ),
            dangerously_flush: false,
            enable_logging: false,
            max_connections: 1,
            min_connections: 1,
            connect_timeout: 500,
            idle_timeout: 500,
            poll_interval_sec: 1,
            num_workers: 1,
        };

        let pool = connect(&qcfg).await.unwrap();
        sqlx::raw_sql(
            r"
        DROP TABLE IF EXISTS sqlt_loco_queue;
        DROP TABLE IF EXISTS sqlt_loco_queue_lock;
        ",
        )
        .execute(&pool)
        .await
        .expect("drop table if exists");

        pool
    }

    async fn get_all_jobs(pool: &SqlitePool) -> Vec<Job> {
        sqlx::query("select * from sqlt_loco_queue")
            .fetch_all(pool)
            .await
            .expect("get jobs")
            .iter()
            .filter_map(|row| to_job(row).ok())
            .collect()
    }

    async fn get_job(pool: &SqlitePool, id: &str) -> Job {
        sqlx::query(&format!("select * from sqlt_loco_queue where id = '{id}'"))
            .fetch_all(pool)
            .await
            .expect("get jobs")
            .first()
            .and_then(|row| to_job(row).ok())
            .expect("job not found")
    }

    #[tokio::test]
    async fn can_initialize_database() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());

        for table in ["sqlt_loco_queue", "sqlt_loco_queue_lock"] {
            let table_info: Vec<TableInfo> =
                query_as::<_, TableInfo>(&format!("PRAGMA table_info({table})"))
                    .fetch_all(&pool)
                    .await
                    .unwrap();

            assert_debug_snapshot!(table, table_info);
        }
    }

    #[tokio::test]
    async fn can_enqueue() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());

        let jobs = get_all_jobs(&pool).await;

        assert_eq!(jobs.len(), 0);

        let run_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2023, 1, 15)
                .unwrap()
                .and_time(NaiveTime::from_hms_opt(12, 30, 0).unwrap()),
        );

        let job_data = serde_json::json!({"user_id": 1});
        assert!(
            enqueue(&pool, "PasswordChangeNotification", job_data, run_at, None)
                .await
                .is_ok()
        );

        let jobs = get_all_jobs(&pool).await;

        assert_eq!(jobs.len(), 1);
        with_settings!({
            filters => reduction().iter().map(|&(pattern, replacement)| (pattern, replacement)),
        }, {
            assert_debug_snapshot!(jobs);
        });

        // validate lock status
        let job_lock: JobQueueLock =
            query_as::<_, JobQueueLock>("select * from sqlt_loco_queue_lock")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert!(!job_lock.is_locked);
    }

    #[tokio::test]
    async fn can_dequeue() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());

        let jobs = get_all_jobs(&pool).await;

        assert_eq!(jobs.len(), 0);

        let run_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2023, 1, 15)
                .unwrap()
                .and_time(NaiveTime::from_hms_opt(12, 30, 0).unwrap()),
        );

        let job_data = serde_json::json!({"user_id": 1});
        assert!(
            enqueue(&pool, "PasswordChangeNotification", job_data, run_at, None)
                .await
                .is_ok()
        );

        let job_before_dequeue = get_all_jobs(&pool)
            .await
            .first()
            .cloned()
            .expect("gets first job");
        assert_eq!(job_before_dequeue.status, JobStatus::Queued);

        std::thread::sleep(std::time::Duration::from_secs(1));

        assert!(dequeue(&pool).await.is_ok());

        let job_after_dequeue = get_all_jobs(&pool)
            .await
            .first()
            .cloned()
            .expect("gets first job");

        assert_ne!(job_after_dequeue.updated_at, job_before_dequeue.updated_at);
        with_settings!({
            filters => reduction().iter().map(|&(pattern, replacement)| (pattern, replacement)),
        }, {
            assert_debug_snapshot!(job_after_dequeue);
        });
    }

    #[tokio::test]
    async fn can_complete_job_without_interval() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        tests_cfg::queue::sqlite_seed_data(&pool).await;

        let job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA99").await;

        assert_eq!(job.status, JobStatus::Queued);
        assert!(complete_job(&pool, &job.id, None).await.is_ok());

        let job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA99").await;

        assert_eq!(job.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn can_complete_job_with_interval() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        tests_cfg::queue::sqlite_seed_data(&pool).await;

        let before_complete_job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA98").await;
        assert_eq!(before_complete_job.status, JobStatus::Completed);

        std::thread::sleep(std::time::Duration::from_secs(1));

        assert!(complete_job(&pool, &before_complete_job.id, Some(10))
            .await
            .is_ok());

        let after_complete_job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA98").await;

        assert_ne!(
            after_complete_job.updated_at,
            before_complete_job.updated_at
        );
        with_settings!({
            filters => reduction().iter().map(|&(pattern, replacement)| (pattern, replacement)),
        }, {
            assert_debug_snapshot!(after_complete_job);
        });
    }

    #[tokio::test]
    async fn can_fail_job() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        tests_cfg::queue::sqlite_seed_data(&pool).await;

        let before_fail_job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA97").await;

        std::thread::sleep(std::time::Duration::from_secs(1));

        assert!(fail_job(
            &pool,
            &before_fail_job.id,
            &crate::Error::string("some error")
        )
        .await
        .is_ok());

        let after_fail_job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA97").await;

        assert_ne!(after_fail_job.updated_at, before_fail_job.updated_at);
        with_settings!({
            filters => reduction().iter().map(|&(pattern, replacement)| (pattern, replacement)),
        }, {
            assert_debug_snapshot!(after_fail_job);
        });
    }

    #[tokio::test]
    async fn can_cancel_job_by_name() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        tests_cfg::queue::sqlite_seed_data(&pool).await;

        let count_cancelled_jobs = get_all_jobs(&pool)
            .await
            .iter()
            .filter(|j| j.status == JobStatus::Cancelled)
            .count();

        assert_eq!(count_cancelled_jobs, 1);

        assert!(cancel_jobs_by_name(&pool, "UserAccountActivation")
            .await
            .is_ok());

        let count_cancelled_jobs = get_all_jobs(&pool)
            .await
            .iter()
            .filter(|j| j.status == JobStatus::Cancelled)
            .count();

        assert_eq!(count_cancelled_jobs, 2);
    }

    #[tokio::test]
    async fn can_clear() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        tests_cfg::queue::sqlite_seed_data(&pool).await;

        let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlt_loco_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        let lock_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlt_loco_queue_lock")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_ne!(job_count, 0);
        assert_ne!(lock_count, 0);

        assert!(clear(&pool).await.is_ok());
        let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlt_loco_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        let lock_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlt_loco_queue_lock")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(job_count, 0);
        assert_eq!(lock_count, 0);
    }

    #[tokio::test]
    async fn can_clear_by_status() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        tests_cfg::queue::sqlite_seed_data(&pool).await;

        let jobs = get_all_jobs(&pool).await;

        assert_eq!(jobs.len(), 14);
        assert_eq!(
            jobs.iter()
                .filter(|j| j.status == JobStatus::Completed)
                .count(),
            3
        );
        assert_eq!(
            jobs.iter()
                .filter(|j| j.status == JobStatus::Failed)
                .count(),
            2
        );

        assert!(
            clear_by_status(&pool, vec![JobStatus::Completed, JobStatus::Failed])
                .await
                .is_ok()
        );
        let jobs = get_all_jobs(&pool).await;

        assert_eq!(jobs.len(), 9);
        assert_eq!(
            jobs.iter()
                .filter(|j| j.status == JobStatus::Completed)
                .count(),
            0
        );
        assert_eq!(
            jobs.iter()
                .filter(|j| j.status == JobStatus::Failed)
                .count(),
            0
        );
    }

    #[tokio::test]
    async fn can_clear_jobs_older_than() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());

        sqlx::query(
            r"INSERT INTO sqlt_loco_queue (id, name, task_data, status,run_at, created_at, updated_at) VALUES
            ('job1', 'Test Job 1', '{}', 'queued', CURRENT_TIMESTAMP,DATETIME('now', '-15 days'), CURRENT_TIMESTAMP),
            ('job2', 'Test Job 2', '{}', 'queued', CURRENT_TIMESTAMP, DATETIME('now', '-5 days'), CURRENT_TIMESTAMP),
            ('job3', 'Test Job 3', '{}', 'queued', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(get_all_jobs(&pool).await.len(), 3);
        assert!(clear_jobs_older_than(&pool, 10, None).await.is_ok());
        assert_eq!(get_all_jobs(&pool).await.len(), 2);
    }

    #[tokio::test]
    async fn can_clear_jobs_older_than_with_status() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());

        sqlx::query(
            r"INSERT INTO sqlt_loco_queue (id, name, task_data, status,run_at, created_at, updated_at) VALUES
            ('job1', 'Test Job 1', '{}', 'completed', CURRENT_TIMESTAMP,DATETIME('now', '-20 days'), CURRENT_TIMESTAMP),
            ('job2', 'Test Job 2', '{}', 'failed', CURRENT_TIMESTAMP,DATETIME('now', '-15 days'), CURRENT_TIMESTAMP),
            ('job3', 'Test Job 3', '{}', 'completed', CURRENT_TIMESTAMP, DATETIME('now', '-5 days'), CURRENT_TIMESTAMP),
            ('job4', 'Test Job 4', '{}', 'cancelled', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(get_all_jobs(&pool).await.len(), 4);
        assert!(clear_jobs_older_than(
            &pool,
            10,
            Some(&vec![JobStatus::Cancelled, JobStatus::Completed])
        )
        .await
        .is_ok());

        assert_eq!(get_all_jobs(&pool).await.len(), 3);
    }

    #[tokio::test]
    async fn can_get_jobs() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;
        assert!(initialize_database(&pool).await.is_ok());
        tests_cfg::queue::sqlite_seed_data(&pool).await;

        assert_eq!(
            get_jobs(&pool, Some(&vec![JobStatus::Failed]), None)
                .await
                .expect("get jobs")
                .len(),
            2
        );
        assert_eq!(
            get_jobs(
                &pool,
                Some(&vec![JobStatus::Failed, JobStatus::Completed]),
                None
            )
            .await
            .expect("get jobs")
            .len(),
            5
        );
        assert_eq!(
            get_jobs(&pool, None, None).await.expect("get jobs").len(),
            14
        );
    }

    #[tokio::test]
    async fn can_get_jobs_with_age() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;
        assert!(initialize_database(&pool).await.is_ok());

        sqlx::query(
            r"INSERT INTO sqlt_loco_queue (id, name, task_data, status,run_at, created_at, updated_at) VALUES
            ('job1', 'Test Job 1', '{}', 'completed', CURRENT_TIMESTAMP,DATETIME('now', '-20 days'), CURRENT_TIMESTAMP),
            ('job2', 'Test Job 2', '{}', 'failed', CURRENT_TIMESTAMP,DATETIME('now', '-15 days'), CURRENT_TIMESTAMP),
            ('job3', 'Test Job 3', '{}', 'completed', CURRENT_TIMESTAMP, DATETIME('now', '-5 days'), CURRENT_TIMESTAMP),
            ('job4', 'Test Job 4', '{}', 'cancelled', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            get_jobs(
                &pool,
                Some(&vec![JobStatus::Failed, JobStatus::Completed]),
                Some(10)
            )
            .await
            .expect("get jobs")
            .len(),
            2
        );
    }

    #[tokio::test]
    async fn can_requeue() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        sqlx::query(
            r"INSERT INTO sqlt_loco_queue (id, name, task_data, status,run_at, created_at, updated_at) VALUES
            ('job1', 'Test Job 1', '{}', 'processing', CURRENT_TIMESTAMP,CURRENT_TIMESTAMP, DATETIME('now', '-20 minute')),
            ('job2', 'Test Job 2', '{}', 'processing', CURRENT_TIMESTAMP,CURRENT_TIMESTAMP, DATETIME('now', '-5 minute')),
            ('job3', 'Test Job 3', '{}', 'completed', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, DATETIME('now', '-5 minute')),
            ('job4', 'Test Job 4', '{}', 'queued', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
            ('job5', 'Test Job 5', '{}', 'processing', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let jobs = get_all_jobs(&pool).await;

        let processing_job_count = jobs
            .iter()
            .filter(|job| job.status == JobStatus::Processing)
            .count();
        let queued_job_count = jobs
            .iter()
            .filter(|job| job.status == JobStatus::Queued)
            .count();

        assert_eq!(processing_job_count, 3);
        assert_eq!(queued_job_count, 1);
        assert!(requeue(&pool, &10).await.is_ok());
        let jobs = get_all_jobs(&pool).await;
        let processing_job_count = jobs
            .iter()
            .filter(|job| job.status == JobStatus::Processing)
            .count();
        let queued_job_count = jobs
            .iter()
            .filter(|job| job.status == JobStatus::Queued)
            .count();

        assert_eq!(processing_job_count, 2);
        assert_eq!(queued_job_count, 2);
    }
}
