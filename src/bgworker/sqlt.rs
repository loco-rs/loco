/// `SQLite` based background job queue provider
use std::time::Duration;

pub use super::sql::{Job, JobData, JobId, JobRegistry, RunOpts};
use super::{
    sql::{to_job, Driver},
    JobHandler, JobStatus, Queue, QueueProvider,
};
use crate::{config::SqliteQueueConfig, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
pub use sqlx::SqlitePool;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
    AssertSqlSafe, ConnectOptions, QueryBuilder,
};
use std::fmt::Write;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};
use ulid::Ulid;

/// [`Driver`] implementation delegating to the `SQLite`-specific
/// `dequeue`/`complete_job`/`fail_job` free functions below.
pub struct SqliteDriver;

impl Driver for SqliteDriver {
    type Pool = SqlitePool;

    fn idle_count(pool: &Self::Pool) -> usize {
        pool.num_idle()
    }

    async fn dequeue(pool: &Self::Pool, tags: &[String]) -> crate::Result<Option<Job>> {
        dequeue(pool, tags).await
    }

    async fn complete_job(
        pool: &Self::Pool,
        id: &JobId,
        interval: Option<i64>,
    ) -> crate::Result<()> {
        complete_job(pool, id, interval).await
    }

    async fn fail_job(pool: &Self::Pool, id: &JobId, error: &crate::Error) -> crate::Result<()> {
        fail_job(pool, id, error).await
    }
}

/// The `SQLite` [`QueueProvider`].
pub struct SqliteQueue {
    pub pool: SqlitePool,
    pub registry: Arc<tokio::sync::Mutex<JobRegistry>>,
    pub run_opts: RunOpts,
    pub token: CancellationToken,
}

#[async_trait]
impl QueueProvider for SqliteQueue {
    async fn enqueue(
        &self,
        class: String,
        _queue: Option<String>,
        args: serde_json::Value,
        tags: Option<Vec<String>>,
        priority: Option<i32>,
    ) -> Result<Option<String>> {
        Ok(Some(
            enqueue(
                &self.pool,
                &class,
                args,
                chrono::Utc::now(),
                None,
                tags,
                priority,
            )
            .await
            .map_err(Box::from)?,
        ))
    }

    async fn enqueue_batch(
        &self,
        class: String,
        _queue: Option<String>,
        jobs: Vec<(serde_json::Value, Option<i32>)>,
        tags: Option<Vec<String>>,
    ) -> Result<Vec<JobId>> {
        enqueue_batch(&self.pool, &class, jobs, tags).await
    }

    async fn register_handler(&self, name: String, handler: JobHandler) -> Result<()> {
        let mut registry = self.registry.lock().await;
        registry.insert_handler(name, handler)
    }

    async fn run(&self, tags: Vec<String>) -> Result<()> {
        if let Some(reaper) = self.run_opts.reaper.clone() {
            let pool = self.pool.clone();
            let token = self.token.clone();
            tokio::spawn(async move {
                let interval = std::time::Duration::from_secs(reaper.interval_seconds);
                loop {
                    tokio::select! {
                        () = token.cancelled() => break,
                        () = tokio::time::sleep(interval) => {
                            if let Err(err) = requeue(&pool, &reaper.age_minutes).await {
                                tracing::error!(error = %err, "reaper: failed to requeue stale jobs");
                            }
                        }
                    }
                }
            });
        }
        let handles = self.registry.lock().await.run::<SqliteDriver>(
            &self.pool,
            &self.run_opts,
            &self.token.clone(),
            &tags,
        );
        super::process_worker_handles(handles).await
    }

    async fn setup(&self) -> Result<()> {
        initialize_database(&self.pool).await.map_err(Box::from)?;
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        clear(&self.pool).await.map_err(Box::from)?;
        Ok(())
    }

    async fn ping(&self) -> Result<()> {
        ping(&self.pool).await.map_err(Box::from)?;
        Ok(())
    }

    async fn get_jobs(
        &self,
        status: Option<&Vec<JobStatus>>,
        age_days: Option<i64>,
    ) -> Result<Vec<Job>> {
        let jobs = get_jobs(&self.pool, status, age_days)
            .await
            .map_err(Box::from)?;
        Ok(jobs)
    }

    async fn cancel_jobs_by_name(&self, name: &str) -> Result<()> {
        cancel_jobs_by_name(&self.pool, name).await
    }

    async fn clear_by_status(&self, status: Vec<JobStatus>) -> Result<()> {
        clear_by_status(&self.pool, status).await
    }

    async fn clear_jobs_older_than(
        &self,
        age_days: i64,
        status: Option<&Vec<JobStatus>>,
    ) -> Result<()> {
        clear_jobs_older_than(&self.pool, age_days, status).await
    }

    async fn retry_failed(&self, id: Option<&str>) -> Result<u64> {
        retry_failed(&self.pool, id).await
    }

    async fn requeue(&self, age_minutes: &i64) -> Result<()> {
        requeue(&self.pool, age_minutes).await
    }

    fn describe(&self) -> String {
        "sqlite queue".to_string()
    }

    fn shutdown(&self) -> Result<()> {
        self.token.cancel();
        Ok(())
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
    debug!("Initializing job database tables");
    sqlx::query(AssertSqlSafe(format!(r"
            CREATE TABLE IF NOT EXISTS sqlt_loco_queue (
                id TEXT NOT NULL,
                name TEXT NOT NULL,
                task_data JSON NOT NULL,
                status TEXT NOT NULL DEFAULT '{}',
                run_at TIMESTAMP NOT NULL,
                interval INTEGER,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                tags JSON,
                priority INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_sqlt_queue_status_run_at ON sqlt_loco_queue(status, run_at);

            DROP TABLE IF EXISTS sqlt_loco_queue_lock;
            ", JobStatus::Queued),
    ))
    .execute(pool)
    .await?;

    // Auto-migrate: add the priority column to pre-existing databases.
    let priority_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM pragma_table_info('sqlt_loco_queue')
            WHERE name = 'priority'
        )",
    )
    .fetch_one(pool)
    .await?;
    if !priority_exists {
        debug!("Adding priority column to existing sqlt_loco_queue table");
        sqlx::query("ALTER TABLE sqlt_loco_queue ADD COLUMN priority INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await?;
    }

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
    tags: Option<Vec<String>>,
    priority: Option<i32>,
) -> Result<JobId> {
    let data = serde_json::to_value(data)?;
    let tags_json = match &tags {
        Some(tags) => Some(serde_json::to_value(tags)?),
        None => None,
    };

    #[allow(clippy::cast_possible_truncation)]
    let interval_ms: Option<i64> = interval.map(|i| i.as_millis() as i64);

    let id = Ulid::new().to_string();
    debug!(job_id = %id, job_name = %name, run_at = %run_at, tags = ?tags, priority = ?priority, "Enqueueing job");
    sqlx::query(
        "INSERT INTO sqlt_loco_queue (id, task_data, name, run_at, interval, tags, priority) VALUES \
         ($1, $2, $3, DATETIME($4), $5, $6, $7)",
    )
    .bind(id.clone())
    .bind(data)
    .bind(name)
    .bind(run_at)
    .bind(interval_ms)
    .bind(tags_json)
    .bind(priority.unwrap_or(0))
    .execute(pool)
    .await?;
    Ok(id)
}

/// Maximum number of jobs per INSERT statement. Each row binds 7 parameters,
/// kept conservatively under `SQLITE_MAX_VARIABLE_NUMBER` (defaults to 32766 on
/// `SQLite` >= 3.32, which `sqlx` bundles). Larger batches are split across
/// multiple statements that all run inside one transaction, so the whole batch
/// is still enqueued atomically regardless of how many chunks it spans.
const ENQUEUE_BATCH_CHUNK_SIZE: usize = 1_000;

/// Enqueue multiple jobs in a single atomic batch.
///
/// Each entry of `jobs` is one job's data paired with its priority (`None`
/// for the default); `tags` apply to every job. The returned IDs are in the
/// same order as `jobs`.
///
/// Every job is inserted inside one transaction: either all of them are
/// enqueued or none are. A failure mid-batch rolls back, so the batch leaves
/// nothing behind and is safe to retry without duplicating jobs.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn enqueue_batch(
    pool: &SqlitePool,
    name: &str,
    jobs: Vec<(JobData, Option<i32>)>,
    tags: Option<Vec<String>>,
) -> Result<Vec<JobId>> {
    if jobs.is_empty() {
        return Ok(Vec::new());
    }

    // Store `run_at` in SQLite's canonical `YYYY-MM-DD HH:MM:SS` text format so
    // it compares correctly against `CURRENT_TIMESTAMP` in `dequeue` — the same
    // normalization the single-job `enqueue` gets from its `DATETIME($4)` wrap.
    // Binding a `DateTime<Utc>` directly would store RFC3339 (`...T...+00:00`),
    // which sorts after `CURRENT_TIMESTAMP` and would hide the job until the
    // next day.
    let run_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let tags_json = match &tags {
        Some(tags) => Some(serde_json::to_value(tags)?),
        None => None,
    };
    let mut ids = Vec::with_capacity(jobs.len());

    debug!(count = jobs.len(), job_name = %name, run_at = %run_at, tags = ?tags, "Batch enqueueing jobs");
    let mut tx = pool.begin().await?;
    for chunk in jobs.chunks(ENQUEUE_BATCH_CHUNK_SIZE) {
        let mut query_builder = sqlx::query_builder::QueryBuilder::<sqlx::Sqlite>::new(
            "INSERT INTO sqlt_loco_queue (id, task_data, name, run_at, interval, tags, priority) ",
        );

        query_builder.push_values(chunk.iter(), |mut b, (data, priority)| {
            let id = Ulid::new().to_string();
            b.push_bind(id.clone())
                .push_bind(data.clone())
                .push_bind(name.to_string())
                .push_bind(run_at.clone())
                .push_bind(None::<i64>)
                .push_bind(tags_json.clone())
                .push_bind(priority.unwrap_or(0));
            ids.push(id);
        });

        query_builder.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;

    Ok(ids)
}

async fn dequeue(client: &SqlitePool, worker_tags: &[String]) -> Result<Option<Job>> {
    // `BEGIN IMMEDIATE` takes SQLite's write lock up front, so the SELECT below
    // and the UPDATE that claims the row cannot interleave with another worker's
    // — two workers can never be handed the same job. A plain `BEGIN` defers the
    // lock until the first write, leaving the SELECT unprotected. Concurrent
    // workers serialize here, waiting out the connection's `busy_timeout`.
    let mut tx = client.begin_with("BEGIN IMMEDIATE").await?;

    let mut query = String::from(
        "SELECT id, name, task_data, status, run_at, interval, tags, priority
        FROM sqlt_loco_queue
        WHERE
            status = ? AND
            run_at <= CURRENT_TIMESTAMP",
    );

    // An untagged worker takes only untagged jobs; a tagged one takes any job
    // carrying at least one of its tags.
    if worker_tags.is_empty() {
        query.push_str(" AND (tags IS NULL)");
    } else {
        let any_tag_matches =
            vec!["json_extract(tags, '$') LIKE ?"; worker_tags.len()].join(" OR ");
        query.push_str(" AND (tags IS NOT NULL) AND (");
        query.push_str(&any_tag_matches);
        query.push(')');
    }

    query.push_str(" ORDER BY priority DESC, run_at, id LIMIT 1");

    let mut db_query = sqlx::query(AssertSqlSafe(query)).bind(JobStatus::Queued.to_string());

    for tag in worker_tags {
        // The tags column is a JSON array; match the quoted element as a substring.
        db_query = db_query.bind(format!("%\"{tag}\"%"));
    }

    let row = db_query
        .map(|row: SqliteRow| to_job(&row).ok())
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

    if let Some(job) = row {
        trace!(job_id = %job.id, job_name = %job.name, job_tags = ?job.tags, "Dequeueing job for processing");
        sqlx::query(
            "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(JobStatus::Processing.to_string())
        .bind(&job.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(job))
    } else {
        trace!("No jobs available for processing");
        tx.rollback().await?;
        Ok(None)
    }
}

async fn complete_job(pool: &SqlitePool, id: &JobId, interval_ms: Option<i64>) -> Result<()> {
    if let Some(interval_ms) = interval_ms {
        let next_run_at = Utc::now() + chrono::Duration::milliseconds(interval_ms);
        trace!(
            job_id = %id,
            status = "queued",
            next_run_at = %next_run_at,
            "Rescheduling recurring job"
        );
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
        trace!(job_id = %id, status = "completed", "Marking job as completed");
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
    debug!(job_id = %id, error = %msg, "Marking job as failed");
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
    debug!(job_name = %name, "Cancelling queued jobs by name");
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
    sqlx::query("DELETE FROM sqlt_loco_queue")
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

    debug!(status = ?status, "Clearing jobs by status");
    sqlx::query(AssertSqlSafe(format!(
        "DELETE FROM sqlt_loco_queue WHERE status IN ({status_in})"
    )))
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

    debug!(age_minutes = age_minutes, "Requeueing stalled jobs");
    sqlx::query(AssertSqlSafe(query))
        .bind(JobStatus::Queued.to_string())
        .bind(JobStatus::Processing.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

/// Moves failed jobs back to [`JobStatus::Queued`], returning how many moved.
///
/// `run_at` is reset to now: a retry is an operator saying "run this again",
/// and a job that failed on a future-dated retry schedule would otherwise sit
/// queued until that time arrives. `requeue` is deliberately separate — it
/// rescues jobs stranded in [`JobStatus::Processing`] by a crashed worker and
/// cannot touch a failed one.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn retry_failed(pool: &SqlitePool, id: Option<&str>) -> Result<u64> {
    let result = if let Some(id) = id {
        debug!(job_id = %id, "Retrying failed job");
        sqlx::query(
            "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP, run_at = \
             CURRENT_TIMESTAMP WHERE status = $2 AND id = $3",
        )
        .bind(JobStatus::Queued.to_string())
        .bind(JobStatus::Failed.to_string())
        .bind(id)
        .execute(pool)
        .await?
    } else {
        debug!("Retrying every failed job");
        sqlx::query(
            "UPDATE sqlt_loco_queue SET status = $1, updated_at = CURRENT_TIMESTAMP, run_at = \
             CURRENT_TIMESTAMP WHERE status = $2",
        )
        .bind(JobStatus::Queued.to_string())
        .bind(JobStatus::Failed.to_string())
        .execute(pool)
        .await?
    };

    Ok(result.rows_affected())
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
    // Match SQLite's stored `created_at` format (`CURRENT_TIMESTAMP` ->
    // `YYYY-MM-DD HH:MM:SS`). Using RFC3339 (`%+`) here produces a `T` separator and
    // offset, which sorts incorrectly against the stored value in a TEXT comparison.
    let threshold_date = cutoff_date.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut query_builder =
        QueryBuilder::<sqlx::Sqlite>::new("DELETE FROM sqlt_loco_queue WHERE created_at <= ");
    query_builder.push_bind(threshold_date);

    if let Some(status_list) = status
        && !status_list.is_empty()
    {
        let status_in = status_list
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<String>>()
            .join(",");

        query_builder.push(format!(" AND status IN ({status_in})"));
    }

    debug!(age_days = age_days, status = ?status, "Clearing older jobs");
    query_builder.build().execute(pool).await?;
    Ok(())
}

/// Ping system
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn ping(pool: &SqlitePool) -> Result<()> {
    super::sql::ping(pool, "sqlt_loco_queue").await
}

/// Builds the [`SqliteQueue`] provider (pool, registry, run options, token)
/// from config. Factored out of [`create_provider`] so tests can inspect the
/// resulting `run_opts` without needing to downcast the opaque [`Queue`].
async fn build_provider(qcfg: &SqliteQueueConfig) -> Result<SqliteQueue> {
    debug!(
        num_workers = qcfg.num_workers,
        poll_interval = qcfg.poll_interval_sec,
        "Creating job queue provider"
    );
    let pool = connect(qcfg).await.map_err(Box::from)?;
    let registry = JobRegistry::new();
    let token = CancellationToken::new();
    Ok(SqliteQueue {
        pool,
        registry: Arc::new(tokio::sync::Mutex::new(registry)),
        run_opts: RunOpts {
            num_workers: qcfg.num_workers,
            poll_interval_sec: qcfg.poll_interval_sec,
            reaper: qcfg.reaper.clone(),
        },
        token,
    })
}

/// Create this provider
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn create_provider(qcfg: &SqliteQueueConfig) -> Result<Queue> {
    Ok(Queue::from_provider(Arc::new(build_provider(qcfg).await?)))
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
        let _ = write!(query, " AND status IN ({status_in})");
    }

    if let Some(age_days) = age_days {
        let cutoff_date = Utc::now() - chrono::Duration::days(age_days);
        // Match SQLite's stored `created_at` format (`YYYY-MM-DD HH:MM:SS`); RFC3339
        // (`%+`) would sort incorrectly against it in a TEXT comparison.
        let threshold_date = cutoff_date.format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = write!(query, " AND created_at <= '{threshold_date}' ");
    }

    debug!(status = ?status, age_days = ?age_days, "Retrieving jobs");
    let rows = sqlx::query(AssertSqlSafe(query)).fetch_all(pool).await?;
    let jobs = rows.iter().filter_map(|row| to_job(row).ok()).collect();
    debug!(job_count = rows.len(), "Retrieved jobs from database");
    Ok(jobs)
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use chrono::{NaiveDate, NaiveTime, TimeZone};
    use insta::{assert_debug_snapshot, with_settings};
    use serde::Serialize;
    use sqlx::{query_as, FromRow, Pool, Sqlite};
    use tokio::time::sleep;

    use super::*;
    use crate::{bgworker::BackgroundWorker, tests_cfg};

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

    fn reduction() -> &'static [(&'static str, &'static str)] {
        &[
            ("[A-Z0-9]{26}", "<REDACTED>"),
            (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", "<REDACTED>"),
        ]
    }

    async fn init(db_path: &Path) -> Pool<Sqlite> {
        let pool = connect(&config(db_path)).await.unwrap();
        sqlx::query("DROP TABLE IF EXISTS sqlt_loco_queue")
            .execute(&pool)
            .await
            .expect("drop table if exists");

        pool
    }

    fn config(db_path: &Path) -> SqliteQueueConfig {
        SqliteQueueConfig {
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
            reaper: None,
        }
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
        sqlx::query(AssertSqlSafe(format!(
            "select * from sqlt_loco_queue where id = '{id}'"
        )))
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

        let table = "sqlt_loco_queue";
        let table_info: Vec<TableInfo> =
            query_as::<_, TableInfo>(AssertSqlSafe(format!("PRAGMA table_info({table})")))
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_debug_snapshot!(table, table_info);
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
        let tags = Some(vec!["email".to_string(), "notification".to_string()]);
        assert!(enqueue(
            &pool,
            "PasswordChangeNotification",
            job_data,
            run_at,
            None,
            tags,
            None
        )
        .await
        .is_ok());

        let jobs = get_all_jobs(&pool).await;

        assert_eq!(jobs.len(), 1);
        with_settings!({
            filters => reduction().iter().map(|&(pattern, replacement)| (pattern, replacement)),
        }, {
            assert_debug_snapshot!(jobs);
        });
    }

    #[tokio::test]
    async fn can_enqueue_batch() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        assert_eq!(get_all_jobs(&pool).await.len(), 0);

        // Mixed per-job priorities: the default (None → 0), a high and a low
        // value. Dequeue order must follow priority, not insertion order.
        let jobs = vec![
            (serde_json::json!({"user_id": 1}), None),
            (serde_json::json!({"user_id": 2}), Some(10)),
            (serde_json::json!({"user_id": 3}), Some(-5)),
        ];
        let ids = enqueue_batch(&pool, "BatchJob", jobs, None)
            .await
            .expect("batch enqueue");
        assert_eq!(ids.len(), 3);
        assert_eq!(get_all_jobs(&pool).await.len(), 3);

        // Returned ids are in input order and carry each job's own priority.
        assert_eq!(get_job(&pool, &ids[0]).await.priority, 0);
        assert_eq!(get_job(&pool, &ids[1]).await.priority, 10);
        assert_eq!(get_job(&pool, &ids[2]).await.priority, -5);

        // Regression guard: run_at must be stored in SQLite's canonical
        // `YYYY-MM-DD HH:MM:SS` format. If it were bound as a raw RFC3339
        // `DateTime<Utc>` (with a `T` separator) the `run_at <= CURRENT_TIMESTAMP`
        // predicate in `dequeue` would never match and this would return None.
        std::thread::sleep(std::time::Duration::from_secs(1));
        for expected_user in [2, 1, 3] {
            let job = dequeue(&pool, &[])
                .await
                .expect("dequeue ok")
                .expect("a batched job must be dequeueable (run_at format regression)");
            assert_eq!(
                job.data.get("user_id"),
                Some(&serde_json::json!(expected_user)),
                "batched jobs must be dequeued by priority"
            );
        }

        // Tagged batches bind tags_json for every row; a worker carrying the
        // tag must see them (a tagless worker deliberately does not).
        let tagged = enqueue_batch(
            &pool,
            "BatchJob",
            vec![(serde_json::json!({"user_id": 4}), None)],
            Some(vec!["email".to_string()]),
        )
        .await
        .expect("tagged batch enqueue");
        assert_eq!(tagged.len(), 1);
        std::thread::sleep(std::time::Duration::from_secs(1));
        let dequeued = dequeue(&pool, &["email".to_string()])
            .await
            .expect("dequeue ok");
        assert!(
            dequeued.is_some(),
            "a tagged batched job must be dequeueable by a matching worker"
        );
    }

    #[tokio::test]
    async fn can_enqueue_batch_across_chunks() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());
        assert_eq!(get_all_jobs(&pool).await.len(), 0);

        // Span more than two chunks so the multi-statement path runs inside one
        // transaction. All rows must commit together (atomic batch) and every
        // generated id must be unique.
        let count = ENQUEUE_BATCH_CHUNK_SIZE * 2 + 1;
        let jobs = (0..count)
            .map(|i| (serde_json::json!({ "user_id": i }), None))
            .collect::<Vec<_>>();

        let ids = enqueue_batch(&pool, "BatchJob", jobs, None)
            .await
            .expect("batch enqueue");
        assert_eq!(ids.len(), count);
        assert_eq!(
            ids.iter().collect::<std::collections::HashSet<_>>().len(),
            count,
            "every batched job id must be unique"
        );
        assert_eq!(get_all_jobs(&pool).await.len(), count);
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
        assert!(enqueue(
            &pool,
            "PasswordChangeNotification",
            job_data,
            run_at,
            None,
            None,
            None
        )
        .await
        .is_ok());

        let job_before_dequeue = get_all_jobs(&pool)
            .await
            .first()
            .cloned()
            .expect("gets first job");
        assert_eq!(job_before_dequeue.status, JobStatus::Queued);

        std::thread::sleep(std::time::Duration::from_secs(1));

        assert!(dequeue(&pool, &[]).await.is_ok());

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
        let run_at_before = job.run_at;
        assert!(complete_job(&pool, &job.id, None).await.is_ok());

        let job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA99").await;

        assert_eq!(job.status, JobStatus::Completed);
        // Completing a one-shot job (no interval) must not rewrite `run_at`:
        // it's inert once the job is done.
        assert_eq!(job.run_at, run_at_before);
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
        // Rescheduling a recurring job (with an interval) legitimately
        // advances `run_at`.
        assert_ne!(after_complete_job.run_at, before_complete_job.run_at);
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
        assert_ne!(job_count, 0);

        assert!(clear(&pool).await.is_ok());
        let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlt_loco_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(job_count, 0);
    }

    /// Two workers polling at once must never be handed the same job. This is
    /// the whole reason `dequeue` opens with `BEGIN IMMEDIATE`; under a plain
    /// `BEGIN` both transactions read the same row before either claims it.
    #[tokio::test]
    async fn concurrent_dequeue_never_hands_out_the_same_job() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let qcfg = SqliteQueueConfig {
            max_connections: 8,
            ..config(&tree_fs.root)
        };
        let pool = connect(&qcfg).await.expect("connect");
        initialize_database(&pool).await.expect("init database");

        const JOBS: usize = 20;
        for i in 0..JOBS {
            enqueue(
                &pool,
                &format!("Job{i}"),
                serde_json::json!({}),
                Utc::now(),
                None,
                None,
                None,
            )
            .await
            .expect("enqueue");
        }

        // More pollers than jobs, so every job is contended and the tail of the
        // run also proves an empty queue stays empty.
        let claimed = futures_util::future::join_all(
            (0..JOBS * 2).map(|_| async { dequeue(&pool, &[]).await.expect("dequeue") }),
        )
        .await;

        let mut ids: Vec<String> = claimed.into_iter().flatten().map(|j| j.id).collect();
        let handed_out = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(handed_out, JOBS, "every job should be claimed exactly once");
        assert_eq!(ids.len(), handed_out, "a job was handed to two workers");
    }

    /// `clear` must leave the queue usable. `dangerously_flush` runs it on
    /// every boot, so a `clear` that breaks `dequeue` silently stops the
    /// worker from ever picking up a job.
    #[tokio::test]
    async fn dequeue_still_works_after_clear() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;
        initialize_database(&pool).await.expect("init database");

        clear(&pool).await.expect("clear");

        enqueue(
            &pool,
            "PostCleanupJob",
            serde_json::json!({}),
            Utc::now(),
            None,
            None,
            None,
        )
        .await
        .expect("enqueue");

        let job = dequeue(&pool, &[]).await.expect("dequeue");
        assert_eq!(
            job.map(|j| j.name),
            Some("PostCleanupJob".to_string()),
            "the queue stopped handing out jobs after clear()"
        );
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
    async fn can_retry_failed_jobs() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;
        assert!(initialize_database(&pool).await.is_ok());

        sqlx::query(
            r"INSERT INTO sqlt_loco_queue (id, name, task_data, status, run_at, created_at, updated_at) VALUES
             ('failed1', 'Failed Job 1', '{}', 'failed', DATETIME('now', '+1 day'), CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
             ('failed2', 'Failed Job 2', '{}', 'failed', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
             ('stuck', 'Stuck Job', '{}', 'processing', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
             ('done', 'Done Job', '{}', 'completed', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
        )
        .execute(&pool)
        .await
        .unwrap();

        // `requeue` is the pre-existing verb and is documented as the recourse
        // for a failed job. It is not: it only touches `processing`.
        requeue(&pool, &0).await.expect("requeue");
        assert_eq!(
            get_jobs(&pool, Some(&vec![JobStatus::Failed]), None)
                .await
                .expect("get jobs")
                .len(),
            2,
            "requeue must leave failed jobs alone"
        );

        assert_eq!(
            retry_failed(&pool, Some("failed1")).await.expect("retry"),
            1
        );
        let failed = get_jobs(&pool, Some(&vec![JobStatus::Failed]), None)
            .await
            .expect("get jobs");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].id, "failed2");

        // `failed1` was scheduled a day out; a retry that left `run_at` alone
        // would queue a job the worker cannot pick up until tomorrow.
        let queued = get_jobs(&pool, Some(&vec![JobStatus::Queued]), None)
            .await
            .expect("get jobs");
        let retried = queued
            .iter()
            .find(|job| job.id == "failed1")
            .expect("the retried job is queued");
        // Tolerance, not sloppiness: `NOW()` is the database's clock, and a
        // containerised Postgres runs ~100ms ahead of the host. The property
        // under test is that `run_at` is no longer a day out, so compare
        // against the schedule it was rescued from rather than against `now`.
        assert!(
            retried.run_at < Utc::now() + chrono::Duration::hours(1),
            "run_at must be reset so the job is immediately runnable, got {}",
            retried.run_at
        );

        assert_eq!(retry_failed(&pool, None).await.expect("retry"), 1);
        assert!(get_jobs(&pool, Some(&vec![JobStatus::Failed]), None)
            .await
            .expect("get jobs")
            .is_empty());

        assert_eq!(
            get_jobs(&pool, Some(&vec![JobStatus::Completed]), None)
                .await
                .expect("get jobs")
                .len(),
            1
        );
        assert_eq!(
            retry_failed(&pool, Some("nonexistent"))
                .await
                .expect("retry"),
            0
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

    #[tokio::test]
    async fn create_provider_wires_reaper_config() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let qcfg = SqliteQueueConfig {
            uri: format!(
                "sqlite://{}?mode=rwc",
                tree_fs.root.join("reaper.sqlite").display()
            ),
            dangerously_flush: false,
            enable_logging: false,
            max_connections: 1,
            min_connections: 1,
            connect_timeout: 500,
            idle_timeout: 500,
            poll_interval_sec: 1,
            num_workers: 1,
            reaper: Some(crate::config::ReaperConfig {
                age_minutes: 5,
                interval_seconds: 30,
            }),
        };

        let provider = build_provider(&qcfg).await.expect("build provider");
        let reaper = provider
            .run_opts
            .reaper
            .expect("reaper should be wired from config");
        assert_eq!(reaper.age_minutes, 5);
        assert_eq!(reaper.interval_seconds, 30);
    }

    #[tokio::test]
    async fn create_provider_defaults_reaper_to_none() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let qcfg = SqliteQueueConfig {
            uri: format!(
                "sqlite://{}?mode=rwc",
                tree_fs.root.join("no_reaper.sqlite").display()
            ),
            dangerously_flush: false,
            enable_logging: false,
            max_connections: 1,
            min_connections: 1,
            connect_timeout: 500,
            idle_timeout: 500,
            poll_interval_sec: 1,
            num_workers: 1,
            reaper: None,
        };

        let provider = build_provider(&qcfg).await.expect("build provider");
        assert!(provider.run_opts.reaper.is_none());
    }

    #[tokio::test]
    async fn can_dequeue_with_priority_ordering() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());

        // Enqueue jobs with different priorities and timestamps.
        // All jobs are in the past so they're ready to be dequeued.
        let base_time = Utc::now() - chrono::Duration::minutes(10);

        // priority 10, later timestamp
        let run_at_1 = base_time + chrono::Duration::minutes(3);
        let job_id_1 = enqueue(
            &pool,
            "Task1",
            serde_json::json!({"task": "low_priority_late"}),
            run_at_1,
            None,
            None,
            Some(10),
        )
        .await
        .expect("enqueue job 1");

        // priority 20, later timestamp (highest priority → first)
        let run_at_2 = base_time + chrono::Duration::minutes(2);
        let job_id_2 = enqueue(
            &pool,
            "Task2",
            serde_json::json!({"task": "high_priority_late"}),
            run_at_2,
            None,
            None,
            Some(20),
        )
        .await
        .expect("enqueue job 2");

        // priority 10, earlier timestamp (same priority, earlier run_at → before job 1)
        let run_at_3 = base_time + chrono::Duration::minutes(1);
        let job_id_3 = enqueue(
            &pool,
            "Task3",
            serde_json::json!({"task": "low_priority_early"}),
            run_at_3,
            None,
            None,
            Some(10),
        )
        .await
        .expect("enqueue job 3");

        // priority 5, earliest timestamp (lowest priority → last)
        let run_at_4 = base_time;
        let job_id_4 = enqueue(
            &pool,
            "Task4",
            serde_json::json!({"task": "lowest_priority_early"}),
            run_at_4,
            None,
            None,
            Some(5),
        )
        .await
        .expect("enqueue job 4");

        // Expected dequeue order: job 2 (prio 20), job 3 (prio 10, earlier),
        // job 1 (prio 10, later), job 4 (prio 5).
        for (expected_id, expected_priority) in [
            (&job_id_2, 20),
            (&job_id_3, 10),
            (&job_id_1, 10),
            (&job_id_4, 5),
        ] {
            let job = dequeue(&pool, &[])
                .await
                .expect("dequeue failed")
                .expect("expected a job");
            assert_eq!(&job.id, expected_id);
            assert_eq!(job.priority, expected_priority);
            complete_job(&pool, &job.id, None)
                .await
                .expect("Failed to complete job");
        }

        // No more jobs
        let job = dequeue(&pool, &[]).await.expect("dequeue failed");
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn can_handle_worker_panic() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());

        let job_data = serde_json::json!(null);
        let job_id = enqueue(&pool, "PanicJob", job_data, Utc::now(), None, None, None)
            .await
            .expect("Failed to enqueue job");

        struct PanicWorker;
        #[async_trait::async_trait]
        impl BackgroundWorker<()> for PanicWorker {
            fn build(_ctx: &crate::app::AppContext) -> Self {
                Self
            }
            async fn perform(&self, _args: ()) -> crate::Result<()> {
                panic!("intentional panic for testing");
            }
        }

        let mut registry = JobRegistry::new();
        let handler = crate::bgworker::erase_worker(PanicWorker);
        assert!(registry
            .insert_handler("PanicJob".to_string(), handler)
            .is_ok());

        // Get the initial job state
        let job = get_job(&pool, &job_id).await;
        assert_eq!(job.status, JobStatus::Queued);

        // Start the worker
        let opts = RunOpts {
            num_workers: 1,
            poll_interval_sec: 1,
            reaper: None,
        };
        let token = CancellationToken::new();
        let handles = registry.run::<SqliteDriver>(&pool, &opts, &token, &[]);

        // Wait a bit for the worker to process the job
        sleep(Duration::from_secs(1)).await;

        // Stop the worker
        for handle in handles {
            handle.abort();
        }

        // Verify the job is marked as failed
        let failed_job = get_job(&pool, &job_id).await;
        assert_eq!(failed_job.status, JobStatus::Failed);

        // Print and verify the error message stored in job data
        println!("Job data: {:?}", failed_job.data);
        let error_msg = failed_job
            .data
            .as_object()
            .and_then(|obj| obj.get("error"))
            .and_then(|v| v.as_str())
            .expect("Expected error message in job data");
        assert!(
            error_msg.contains("intentional panic for testing"),
            "Error message '{error_msg}' did not contain expected text"
        );
    }

    #[tokio::test]
    async fn can_dequeue_with_tags() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let pool = init(&tree_fs.root).await;

        assert!(initialize_database(&pool).await.is_ok());

        // Add a job with email tag
        let run_at = Utc::now() - chrono::Duration::minutes(5); // In the past so it's ready to process
        let job_data = serde_json::json!({"user_id": 1});
        let email_tags = Some(vec!["email".to_string()]);

        // Insert email job
        let email_id = enqueue(
            &pool,
            "EmailNotification",
            job_data.clone(),
            run_at,
            None,
            email_tags,
            None,
        )
        .await
        .expect("Failed to enqueue email job");

        // Insert job with "sms" tag
        let sms_tags = Some(vec!["sms".to_string()]);
        let sms_id = enqueue(
            &pool,
            "SmsNotification",
            job_data.clone(),
            run_at,
            None,
            sms_tags,
            None,
        )
        .await
        .expect("Failed to enqueue sms job");

        // Insert job with multiple tags
        let multi_tags = Some(vec!["email".to_string(), "priority".to_string()]);
        let multi_id = enqueue(
            &pool,
            "PriorityEmail",
            job_data.clone(),
            run_at,
            None,
            multi_tags,
            None,
        )
        .await
        .expect("Failed to enqueue multi-tag job");

        // Insert job with no tags
        let no_tag_id = enqueue(
            &pool,
            "GenericNotification",
            job_data.clone(),
            run_at,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to enqueue untagged job");

        // Verify all jobs are in the database
        let all_jobs = get_all_jobs(&pool).await;
        assert_eq!(all_jobs.len(), 4);

        // 1. Worker with no tags should only get untagged jobs
        let job = dequeue(&pool, &[]).await.expect("dequeue failed");
        assert!(job.is_some());
        let job = job.unwrap();
        assert_eq!(job.id, no_tag_id);
        assert!(job.tags.is_none());

        // Mark the job as completed to remove it from the queued items
        complete_job(&pool, &job.id, None)
            .await
            .expect("Failed to complete job");

        // 2. Worker with "email" tag should get one of the email-tagged jobs
        let job = dequeue(&pool, &["email".to_string()])
            .await
            .expect("dequeue failed");
        assert!(job.is_some());
        let job = job.unwrap();
        assert!(
            job.id == email_id || job.id == multi_id,
            "Expected either email job or multi-tag job"
        );
        assert!(job.tags.is_some());
        assert!(job.tags.as_ref().unwrap().contains(&"email".to_string()));

        // Mark the job as completed
        complete_job(&pool, &job.id, None)
            .await
            .expect("Failed to complete job");

        // 3. Worker with "email" tag should get the remaining email job
        let job = dequeue(&pool, &["email".to_string()])
            .await
            .expect("dequeue failed");
        assert!(job.is_some());
        let job = job.unwrap();
        assert!(
            job.id == email_id || job.id == multi_id,
            "Expected either email job or multi-tag job"
        );
        assert!(job.tags.is_some());
        assert!(job.tags.as_ref().unwrap().contains(&"email".to_string()));

        // Mark the job as completed
        complete_job(&pool, &job.id, None)
            .await
            .expect("Failed to complete job");

        // 4. Worker with "sms" tag should get the sms job
        let job = dequeue(&pool, &["sms".to_string()])
            .await
            .expect("dequeue failed");
        assert!(job.is_some());
        let job = job.unwrap();
        assert_eq!(job.id, sms_id);
        assert!(job.tags.is_some());
        assert_eq!(job.tags.as_ref().unwrap(), &vec!["sms".to_string()]);

        // Mark the job as completed
        complete_job(&pool, &job.id, None)
            .await
            .expect("Failed to complete job");

        // 5. No more jobs should be available
        let job = dequeue(&pool, &["email".to_string()])
            .await
            .expect("dequeue failed");
        assert!(job.is_none());

        // 6. No more jobs should be available for untagged worker
        let job = dequeue(&pool, &[]).await.expect("dequeue failed");
        assert!(job.is_none());
    }
}
