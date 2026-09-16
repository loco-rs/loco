/// Postgres based background job queue provider
use std::time::Duration;

pub use super::sql::{Job, JobData, JobId, JobRegistry, RunOpts};
use super::{
    sql::{to_job, Driver},
    JobHandler, JobStatus, Queue, QueueProvider,
};
use crate::{config::PostgresQueueConfig, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
pub use sqlx::PgPool;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgRow},
    AssertSqlSafe, ConnectOptions,
};
use std::fmt::Write;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};
use ulid::Ulid;

/// [`Driver`] implementation delegating to the Postgres-specific
/// `dequeue`/`complete_job`/`fail_job` free functions below.
pub struct PgDriver;

impl Driver for PgDriver {
    type Pool = PgPool;

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

/// The Postgres [`QueueProvider`].
pub struct PgQueue {
    pub pool: PgPool,
    pub registry: Arc<tokio::sync::Mutex<JobRegistry>>,
    pub run_opts: RunOpts,
    pub token: CancellationToken,
}

#[async_trait]
impl QueueProvider for PgQueue {
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
        enqueue_batch(&self.pool, &class, jobs, chrono::Utc::now(), tags).await
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
        let handles = self.registry.lock().await.run::<PgDriver>(
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
        get_jobs(&self.pool, status, age_days).await
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
        "postgres queue".to_string()
    }

    fn shutdown(&self) -> Result<()> {
        self.token.cancel();
        Ok(())
    }
}

async fn connect(cfg: &PostgresQueueConfig) -> Result<PgPool> {
    let mut conn_opts: PgConnectOptions = cfg.uri.parse()?;
    if !cfg.enable_logging {
        conn_opts = conn_opts.disable_statement_logging();
    }
    let pool = PgPoolOptions::new()
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
pub async fn initialize_database(pool: &PgPool) -> Result<()> {
    debug!("Initializing job database tables");

    // Check if the table already exists.
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_name = 'pg_loco_queue'
        )",
    )
    .fetch_one(pool)
    .await?;

    if table_exists {
        // Auto-migrate: add the priority column to pre-existing tables.
        let priority_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT FROM information_schema.columns
                WHERE table_name = 'pg_loco_queue'
                AND column_name = 'priority'
            )",
        )
        .fetch_one(pool)
        .await?;

        if !priority_exists {
            debug!("Adding priority column to existing pg_loco_queue table");
            sqlx::query("ALTER TABLE pg_loco_queue ADD COLUMN priority INT NOT NULL DEFAULT 0")
                .execute(pool)
                .await?;
        }
    } else {
        sqlx::raw_sql(AssertSqlSafe(format!(
            r"
                CREATE TABLE pg_loco_queue (
                    id VARCHAR NOT NULL,
                    name VARCHAR NOT NULL,
                    task_data JSONB NOT NULL,
                    status VARCHAR NOT NULL DEFAULT '{}',
                    run_at TIMESTAMPTZ NOT NULL,
                    interval BIGINT,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    tags JSONB,
                    priority INT NOT NULL DEFAULT 0
                );
                ",
            JobStatus::Queued
        )))
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
    pool: &PgPool,
    name: &str,
    data: JobData,
    run_at: DateTime<Utc>,
    interval: Option<Duration>,
    tags: Option<Vec<String>>,
    priority: Option<i32>,
) -> Result<JobId> {
    let data_json = serde_json::to_value(data)?;
    let tags_json = match &tags {
        Some(tags) => Some(serde_json::to_value(tags)?),
        None => None,
    };

    #[allow(clippy::cast_possible_truncation)]
    let interval_ms: Option<i64> = interval.map(|i| i.as_millis() as i64);

    let id = Ulid::new().to_string();
    debug!(job_id = %id, job_name = %name, run_at = %run_at, tags = ?tags, "Enqueueing job");
    sqlx::query(
        "INSERT INTO pg_loco_queue (id, task_data, name, run_at, interval, tags, priority) VALUES \
         ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id.clone())
    .bind(data_json)
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
/// so this stays well under Postgres's 65535 bind-parameter cap and bounds the
/// statement size (mirrors Sidekiq's bulk-push chunking). Larger batches are
/// split across multiple statements that all run inside one transaction, so
/// the whole batch is still enqueued atomically regardless of how many chunks
/// it spans.
const ENQUEUE_BATCH_CHUNK_SIZE: usize = 5_000;

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
    pool: &PgPool,
    name: &str,
    jobs: Vec<(JobData, Option<i32>)>,
    run_at: DateTime<Utc>,
    tags: Option<Vec<String>>,
) -> Result<Vec<JobId>> {
    if jobs.is_empty() {
        return Ok(Vec::new());
    }

    let tags_json = match &tags {
        Some(tags) => Some(serde_json::to_value(tags)?),
        None => None,
    };
    let mut ids = Vec::with_capacity(jobs.len());

    debug!(count = jobs.len(), job_name = %name, run_at = %run_at, tags = ?tags, "Batch enqueueing jobs");
    let mut tx = pool.begin().await?;
    for chunk in jobs.chunks(ENQUEUE_BATCH_CHUNK_SIZE) {
        let mut query_builder = sqlx::query_builder::QueryBuilder::<sqlx::Postgres>::new(
            "INSERT INTO pg_loco_queue (id, task_data, name, run_at, interval, tags, priority) ",
        );

        query_builder.push_values(chunk.iter(), |mut b, (data, priority)| {
            let id = Ulid::new().to_string();
            b.push_bind(id.clone())
                .push_bind(data.clone())
                .push_bind(name.to_string())
                .push_bind(run_at)
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

async fn dequeue(client: &PgPool, worker_tags: &[String]) -> Result<Option<Job>> {
    let mut tx = client.begin().await?;

    let mut query = String::from(
        "SELECT id, name, task_data, status, run_at, interval, tags, priority FROM pg_loco_queue WHERE status = $1 AND run_at <= NOW() ",
    );

    // An untagged worker takes only untagged jobs; a tagged one takes any job
    // carrying at least one of its tags. `?` is jsonb's "array contains this
    // string" operator, one bind per tag starting at $2.
    if worker_tags.is_empty() {
        query.push_str("AND (tags IS NULL) ");
    } else {
        let any_tag_matches = (0..worker_tags.len())
            .map(|i| format!("(tags)::jsonb ? ${}", i + 2))
            .collect::<Vec<_>>()
            .join(" OR ");
        query.push_str("AND (tags IS NOT NULL) AND (");
        query.push_str(&any_tag_matches);
        query.push(')');
    }

    query.push_str(" ORDER BY priority DESC, run_at, id LIMIT 1 FOR UPDATE SKIP LOCKED");

    // Create the query
    let mut db_query = sqlx::query(AssertSqlSafe(query)).bind(JobStatus::Queued.to_string());

    // Bind tag parameters
    for tag in worker_tags {
        db_query = db_query.bind(tag);
    }

    let row = db_query
        .map(|row: PgRow| to_job(&row).ok())
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

    if let Some(job) = row {
        trace!(job_id = %job.id, job_name = %job.name, job_tags = ?job.tags, "Dequeueing job for processing");
        sqlx::query("UPDATE pg_loco_queue SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(JobStatus::Processing.to_string())
            .bind(&job.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(Some(job))
    } else {
        Ok(None)
    }
}

async fn complete_job(pool: &PgPool, id: &JobId, interval_ms: Option<i64>) -> Result<()> {
    if let Some(interval_ms) = interval_ms {
        let next_run_at = Utc::now() + chrono::Duration::milliseconds(interval_ms);
        trace!(
            job_id = %id,
            status = "queued",
            run_at = %next_run_at,
            "Rescheduling recurring job"
        );
        sqlx::query(
            "UPDATE pg_loco_queue SET status = $1, updated_at = NOW(), run_at = $2 WHERE id = $3",
        )
        .bind(JobStatus::Queued.to_string())
        .bind(next_run_at)
        .bind(id)
        .execute(pool)
        .await?;
    } else {
        trace!(job_id = %id, status = "completed", "Marking job as completed");
        sqlx::query("UPDATE pg_loco_queue SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(JobStatus::Completed.to_string())
            .bind(id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

async fn fail_job(pool: &PgPool, id: &JobId, error: &crate::Error) -> Result<()> {
    let msg = error.to_string();
    debug!(job_id = %id, error = %msg, "Marking job as failed");
    let error_json = serde_json::json!({ "error": msg });
    sqlx::query(
        "UPDATE pg_loco_queue SET status = $1, updated_at = NOW(), task_data = task_data || \
         $2::jsonb WHERE id = $3",
    )
    .bind(JobStatus::Failed.to_string())
    .bind(error_json)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Cancels jobs in the `pg_loco_queue` table by their name.
///
/// This function updates the status of all jobs with the given `name` and a status of
/// [`JobStatus::Queued`] to [`JobStatus::Cancelled`]. The update also sets the `updated_at` timestamp to the
/// current time.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn cancel_jobs_by_name(pool: &PgPool, name: &str) -> Result<()> {
    debug!(job_name = %name, "Cancelling queued jobs by name");
    sqlx::query(
        "UPDATE pg_loco_queue SET status = $1, updated_at = NOW() WHERE name = $2 AND status = $3",
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
pub async fn clear(pool: &PgPool) -> Result<()> {
    sqlx::query("DELETE FROM pg_loco_queue")
        .execute(pool)
        .await?;
    Ok(())
}

/// Deletes jobs from the `pg_loco_queue` table based on their status.
///
/// This function removes all jobs with a status that matches any of the statuses provided
/// in the `status` argument. The statuses are checked against the `status` column in the
/// database, and any matching rows are deleted.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear_by_status(pool: &PgPool, status: Vec<JobStatus>) -> Result<()> {
    let status_in = status
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>();

    debug!(status = ?status, "Clearing jobs by status");
    sqlx::query("DELETE FROM pg_loco_queue WHERE status = ANY($1)")
        .bind(status_in)
        .execute(pool)
        .await?;
    Ok(())
}

/// Deletes jobs from the `pg_loco_queue` table that are older than a specified number of days.
///
/// This function removes jobs that have a `created_at` timestamp older than the provided
/// number of days. Additionally, if a `status` is provided, only jobs with a status matching
/// one of the provided values will be deleted.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear_jobs_older_than(
    pool: &PgPool,
    age_days: i64,
    status: Option<&Vec<JobStatus>>,
) -> Result<()> {
    let mut query_builder = sqlx::query_builder::QueryBuilder::<sqlx::Postgres>::new(
        "DELETE FROM pg_loco_queue WHERE created_at < NOW() - INTERVAL '1 day' * ",
    );

    query_builder.push_bind(age_days);

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

/// Requeues jobs from [`JobStatus::Processing`] to [`JobStatus::Queued`].
///
/// This function updates the status of all jobs that are currently in the [`JobStatus::Processing`] state
/// to the [`JobStatus::Queued`] state, provided they have been updated more than the specified age (`age_minutes`).
/// The jobs that meet the criteria will have their `updated_at` timestamp set to the current time.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn requeue(pool: &PgPool, age_minutes: &i64) -> Result<()> {
    let interval = format!("{age_minutes} MINUTE");

    let query = format!(
        "UPDATE pg_loco_queue SET status = $1, updated_at = NOW() WHERE status = $2 AND updated_at <= NOW() - INTERVAL '{interval}'"
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
pub async fn retry_failed(pool: &PgPool, id: Option<&str>) -> Result<u64> {
    let result = if let Some(id) = id {
        debug!(job_id = %id, "Retrying failed job");
        sqlx::query(
            "UPDATE pg_loco_queue SET status = $1, updated_at = NOW(), run_at = NOW() WHERE \
             status = $2 AND id::text = $3",
        )
        .bind(JobStatus::Queued.to_string())
        .bind(JobStatus::Failed.to_string())
        .bind(id)
        .execute(pool)
        .await?
    } else {
        debug!("Retrying every failed job");
        sqlx::query(
            "UPDATE pg_loco_queue SET status = $1, updated_at = NOW(), run_at = NOW() WHERE \
             status = $2",
        )
        .bind(JobStatus::Queued.to_string())
        .bind(JobStatus::Failed.to_string())
        .execute(pool)
        .await?
    };

    Ok(result.rows_affected())
}

/// Ping system
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn ping(pool: &PgPool) -> Result<()> {
    super::sql::ping(pool, "pg_loco_queue").await
}

/// Retrieves a list of jobs from the `pg_loco_queue` table in the database.
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
    pool: &PgPool,
    status: Option<&Vec<JobStatus>>,
    age_days: Option<i64>,
) -> Result<Vec<Job>> {
    let mut query = String::from("SELECT * FROM pg_loco_queue where true");

    if let Some(status) = status {
        let status_in = status
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<String>>()
            .join(",");
        let _ = write!(query, " AND status in ({status_in})");
    }

    if let Some(age_days) = age_days {
        let _ = write!(
            query,
            " AND created_at <= NOW() - INTERVAL '1 day' * {age_days}"
        );
    }

    debug!(status = ?status, age_days = ?age_days, "Retrieving jobs");
    let rows = sqlx::query(AssertSqlSafe(query)).fetch_all(pool).await?;
    let jobs = rows.iter().filter_map(|row| to_job(row).ok()).collect();
    debug!(job_count = rows.len(), "Retrieved jobs from database");
    Ok(jobs)
}

/// Builds the [`PgQueue`] provider (pool, registry, run options, token) from
/// config. Factored out of [`create_provider`] so tests can inspect the
/// resulting `run_opts` without needing to downcast the opaque [`Queue`].
async fn build_provider(qcfg: &PostgresQueueConfig) -> Result<PgQueue> {
    debug!(
        num_workers = qcfg.num_workers,
        poll_interval = qcfg.poll_interval_sec,
        "Creating job queue provider"
    );
    let pool = connect(qcfg).await.map_err(Box::from)?;
    let registry = JobRegistry::new();
    let token = CancellationToken::new(); // Create the token
    Ok(PgQueue {
        pool,
        registry: Arc::new(tokio::sync::Mutex::new(registry)),
        run_opts: RunOpts {
            num_workers: qcfg.num_workers,
            poll_interval_sec: qcfg.poll_interval_sec,
            reaper: qcfg.reaper.clone(),
        },
        token, // Pass the token
    })
}

/// Create this provider
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn create_provider(qcfg: &PostgresQueueConfig) -> Result<Queue> {
    Ok(Queue::from_provider(Arc::new(build_provider(qcfg).await?)))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime, TimeZone};
    use insta::{assert_debug_snapshot, with_settings};
    use serde::Serialize;
    use sqlx::{query_as, FromRow};
    use tokio::time::sleep;

    use super::*;
    use crate::{
        bgworker::BackgroundWorker,
        tests_cfg::{self, postgres::setup_postgres_container},
    };

    fn reduction() -> &'static [(&'static str, &'static str)] {
        &[
            ("[A-Z0-9]{26}", "<REDACTED>"),
            (
                r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z",
                "<REDACTED>",
            ),
        ]
    }

    #[derive(Debug, Serialize, FromRow)]
    pub struct TableInfo {
        pub table_schema: Option<String>,
        pub column_name: Option<String>,
        pub column_default: Option<String>,
        pub is_nullable: Option<String>,
        pub data_type: Option<String>,
        pub is_updatable: Option<String>,
    }

    async fn get_all_jobs(pool: &PgPool) -> Vec<Job> {
        sqlx::query("select * from pg_loco_queue")
            .fetch_all(pool)
            .await
            .expect("get jobs")
            .iter()
            .filter_map(|row| to_job(row).ok())
            .collect()
    }

    async fn get_job(pool: &PgPool, id: &str) -> Job {
        sqlx::query(AssertSqlSafe(format!(
            "select * from pg_loco_queue where id = '{id}'"
        )))
        .fetch_all(pool)
        .await
        .expect("get jobs")
        .first()
        .and_then(|row| to_job(row).ok())
        .expect("job not found")
    }

    // New setup function that uses our testcontainer
    async fn setup_pg_test() -> (
        PgPool,
        testcontainers::ContainerAsync<testcontainers::GenericImage>,
    ) {
        let (pg_url, container) = setup_postgres_container().await;
        let pool = PgPool::connect(&pg_url)
            .await
            .expect("Failed to connect to PostgreSQL");

        // Initialize the database
        initialize_database(&pool)
            .await
            .expect("Failed to initialize database");

        (pool, container)
    }

    #[tokio::test]
    async fn can_initialize_database() {
        let (pool, _container) = setup_pg_test().await;

        let table_info: Vec<TableInfo> = query_as::<_, TableInfo>(
            "SELECT * FROM information_schema.columns WHERE table_name =
    'pg_loco_queue'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_debug_snapshot!(table_info);
    }

    #[tokio::test]
    async fn can_enqueue() {
        let (pool, _container) = setup_pg_test().await;

        let jobs = get_all_jobs(&pool).await;

        assert_eq!(jobs.len(), 0);

        let run_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2023, 1, 15)
                .unwrap()
                .and_time(NaiveTime::from_hms_opt(12, 30, 0).unwrap()),
        );

        let job_data: JobData = serde_json::json!({"user_id": 1});
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

        let jobs = get_all_jobs(&pool).await;

        assert_eq!(jobs.len(), 1);
        with_settings!({
                filters => reduction().iter().map(|&(pattern, replacement)|
        (pattern, replacement)),     }, {
                assert_debug_snapshot!(jobs);
            });
    }

    #[tokio::test]
    async fn can_enqueue_batch() {
        let (pool, _container) = setup_pg_test().await;
        assert_eq!(get_all_jobs(&pool).await.len(), 0);

        // Mixed per-job priorities: the default (None → 0), a high and a low
        // value. Dequeue order must follow priority, not insertion order.
        let run_at = Utc::now() - chrono::Duration::minutes(1);
        let jobs = vec![
            (serde_json::json!({"user_id": 1}), None),
            (serde_json::json!({"user_id": 2}), Some(10)),
            (serde_json::json!({"user_id": 3}), Some(-5)),
        ];
        let ids = enqueue_batch(&pool, "BatchJob", jobs, run_at, None)
            .await
            .expect("batch enqueue");
        assert_eq!(ids.len(), 3);
        assert_eq!(get_all_jobs(&pool).await.len(), 3);

        // Returned ids are in input order and carry each job's own priority.
        assert_eq!(get_job(&pool, &ids[0]).await.priority, 0);
        assert_eq!(get_job(&pool, &ids[1]).await.priority, 10);
        assert_eq!(get_job(&pool, &ids[2]).await.priority, -5);

        for expected_user in [2, 1, 3] {
            let job = dequeue(&pool, &[])
                .await
                .expect("dequeue ok")
                .expect("a batched job must be dequeueable");
            assert_eq!(
                job.data.get("user_id"),
                Some(&serde_json::json!(expected_user)),
                "batched jobs must be dequeued by priority"
            );
            complete_job(&pool, &job.id, None)
                .await
                .expect("complete job");
        }
        assert!(dequeue(&pool, &[]).await.expect("dequeue ok").is_none());

        // Tagged batches bind tags_json for every row; a worker carrying the
        // tag must see them and a tagless worker must not.
        let tagged = enqueue_batch(
            &pool,
            "BatchJob",
            vec![(serde_json::json!({"user_id": 4}), None)],
            run_at,
            Some(vec!["email".to_string()]),
        )
        .await
        .expect("tagged batch enqueue");
        assert_eq!(tagged.len(), 1);
        assert!(
            dequeue(&pool, &[]).await.expect("dequeue ok").is_none(),
            "an untagged worker must not see tagged jobs"
        );
        let job = dequeue(&pool, &["email".to_string()])
            .await
            .expect("dequeue ok")
            .expect("a tagged batched job must be dequeueable by a matching worker");
        assert_eq!(job.id, tagged[0]);
        assert_eq!(job.tags, Some(vec!["email".to_string()]));
    }

    #[tokio::test]
    async fn can_enqueue_batch_across_chunks() {
        let (pool, _container) = setup_pg_test().await;
        assert_eq!(get_all_jobs(&pool).await.len(), 0);

        // Span more than two chunks so the multi-statement path runs inside one
        // transaction. All rows must commit together (atomic batch) and every
        // generated id must be unique.
        let count = ENQUEUE_BATCH_CHUNK_SIZE * 2 + 1;
        let jobs = (0..count)
            .map(|i| (serde_json::json!({ "user_id": i }), None))
            .collect::<Vec<_>>();

        let ids = enqueue_batch(&pool, "BatchJob", jobs, Utc::now(), None)
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
        let (pool, _container) = setup_pg_test().await;

        let run_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2023, 1, 15)
                .unwrap()
                .and_time(NaiveTime::from_hms_opt(12, 30, 0).unwrap()),
        );

        let job_data: JobData = serde_json::json!({"user_id": 1});
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
                filters => reduction().iter().map(|&(pattern, replacement)|
        (pattern, replacement)),     }, {
                assert_debug_snapshot!(job_after_dequeue);
            });
    }

    #[tokio::test]
    async fn can_complete_job_without_interval() {
        let (pool, _container) = setup_pg_test().await;
        tests_cfg::queue::postgres_seed_data(&pool).await;

        let job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA99").await;

        assert_eq!(job.status, JobStatus::Queued);
        let run_at_before = job.run_at;
        assert!(complete_job(&pool, &job.id, None).await.is_ok());

        let job = get_job(&pool, "01JDM0X8EVAM823JZBGKYNBA99").await;

        assert_eq!(job.status, JobStatus::Completed);
        // Completing a one-shot job (no interval) must not rewrite `run_at`:
        // it's inert once the job is done, and should match the SQLite
        // backend's behavior.
        assert_eq!(job.run_at, run_at_before);
    }

    #[tokio::test]
    async fn can_complete_job_with_interval() {
        let (pool, _container) = setup_pg_test().await;
        tests_cfg::queue::postgres_seed_data(&pool).await;

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
                filters => reduction().iter().map(|&(pattern, replacement)| (pattern,
        replacement)),     }, {
                assert_debug_snapshot!(after_complete_job);
            });
    }

    #[tokio::test]
    async fn can_fail_job() {
        let (pool, _container) = setup_pg_test().await;
        tests_cfg::queue::postgres_seed_data(&pool).await;

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
                filters => reduction().iter().map(|&(pattern, replacement)| (pattern,
        replacement)),     }, {
                assert_debug_snapshot!(after_fail_job);
            });
    }

    #[tokio::test]
    async fn can_cancel_job_by_name() {
        let (pool, _container) = setup_pg_test().await;
        tests_cfg::queue::postgres_seed_data(&pool).await;

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
        let (pool, _container) = setup_pg_test().await;
        tests_cfg::queue::postgres_seed_data(&pool).await;

        let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_loco_queue")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_ne!(job_count, 0);

        assert!(clear(&pool).await.is_ok());
        let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_loco_queue")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(job_count, 0);
    }

    #[tokio::test]
    async fn can_clear_by_status() {
        let (pool, _container) = setup_pg_test().await;
        tests_cfg::queue::postgres_seed_data(&pool).await;

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
        let (pool, _container) = setup_pg_test().await;

        sqlx::query(
           r"INSERT INTO pg_loco_queue (id, name, task_data, status, run_at,created_at, updated_at) VALUES
             ('job1', 'Test Job 1', '{}', 'queued', NOW(), NOW() - INTERVAL '15days', NOW()),
             ('job2', 'Test Job 2', '{}', 'queued', NOW(),NOW() - INTERVAL '5 days', NOW()),
             ('job3', 'Test Job 3', '{}','queued', NOW(), NOW(), NOW())"
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
        let (pool, _container) = setup_pg_test().await;

        sqlx::query(
           r"INSERT INTO pg_loco_queue (id, name, task_data, status, run_at,created_at, updated_at) VALUES
             ('job1', 'Test Job 1', '{}', 'completed', NOW(), NOW() - INTERVAL '20days', NOW()),
             ('job2', 'Test Job 2', '{}', 'failed', NOW(),NOW() - INTERVAL '15 days', NOW()),
             ('job3', 'Test Job 3', '{}', 'completed', NOW(),NOW() - INTERVAL '5 days', NOW()),
             ('job4', 'Test Job 3', '{}','cancelled', NOW(), NOW(), NOW())"
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
        let (pool, _container) = setup_pg_test().await;
        tests_cfg::queue::postgres_seed_data(&pool).await;

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
        let (pool, _container) = setup_pg_test().await;

        sqlx::query(
            r"INSERT INTO pg_loco_queue (id, name, task_data, status, run_at,created_at, updated_at) VALUES
             ('job1', 'Test Job 1', '{}', 'completed', NOW(), NOW() - INTERVAL '20days', NOW()),
             ('job2', 'Test Job 2', '{}', 'failed', NOW(),NOW() - INTERVAL '15 days', NOW()),
             ('job3', 'Test Job 3', '{}', 'completed', NOW(),NOW() - INTERVAL '5 days', NOW()),
             ('job4', 'Test Job 3', '{}','cancelled', NOW(), NOW(), NOW())"
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            get_jobs(
                &pool,
                Some(&vec![JobStatus::Failed, JobStatus::Completed]),
                Some(11)
            )
            .await
            .expect("get jobs")
            .len(),
            2
        );
    }

    #[tokio::test]
    async fn can_retry_failed_jobs() {
        let (pool, _container) = setup_pg_test().await;

        sqlx::query(
            r"INSERT INTO pg_loco_queue (id, name, task_data, status, run_at, created_at, updated_at) VALUES
             ('failed1', 'Failed Job 1', '{}', 'failed', NOW() + INTERVAL '1 day', NOW(), NOW()),
             ('failed2', 'Failed Job 2', '{}', 'failed', NOW(), NOW(), NOW()),
             ('stuck', 'Stuck Job', '{}', 'processing', NOW(), NOW(), NOW()),
             ('done', 'Done Job', '{}', 'completed', NOW(), NOW(), NOW())"
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

        // Nothing else moved.
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
        let (pool, _container) = setup_pg_test().await;

        sqlx::query(
            r"INSERT INTO pg_loco_queue (id, name, task_data, status, run_at,created_at, updated_at) VALUES
             ('job1', 'Test Job 1', '{}', 'processing', NOW(),NOW(), NOW() - INTERVAL '20 minutes'),
             ('job2', 'Test Job 2', '{}', 'processing', NOW(),NOW(), NOW() - INTERVAL '5 minutes'),
             ('job3', 'Test Job 3', '{}', 'completed', NOW(),NOW(),NOW() - INTERVAL '5 minutes'),
             ('job4', 'Test Job 4', '{}', 'queued', NOW(),NOW(), NOW()),
             ('job4', 'Test Job 5', '{}', 'processing', NOW(), NOW(), NOW())"
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            get_jobs(&pool, Some(&vec![JobStatus::Processing]), None)
                .await
                .expect("get jobs")
                .len(),
            3
        );
        assert_eq!(
            get_jobs(&pool, Some(&vec![JobStatus::Queued]), None)
                .await
                .expect("get jobs")
                .len(),
            1
        );

        requeue(&pool, &10).await.expect("update jobs");

        assert_eq!(
            get_jobs(&pool, Some(&vec![JobStatus::Processing]), None)
                .await
                .expect("get jobs")
                .len(),
            2
        );
        assert_eq!(
            get_jobs(&pool, Some(&vec![JobStatus::Queued]), None)
                .await
                .expect("get jobs")
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn create_provider_wires_reaper_config() {
        let (pg_url, _container) = tests_cfg::postgres::setup_postgres_container().await;
        let qcfg = PostgresQueueConfig {
            uri: pg_url,
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
        let (pg_url, _container) = tests_cfg::postgres::setup_postgres_container().await;
        let qcfg = PostgresQueueConfig {
            uri: pg_url,
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
    async fn can_dequeue_with_priority_extremes_and_ties() {
        let (pool, _container) = setup_pg_test().await;
        let base_time = Utc::now() - chrono::Duration::minutes(10);

        let scenarios = [
            ("PriorityMax", i32::MAX, 4_i64, 1_i32),
            ("PriorityTieEarly", 42_i32, 1_i64, 2_i32),
            ("PriorityTieLate", 42_i32, 3_i64, 3_i32),
            ("PriorityZero", 0_i32, 0_i64, 4_i32),
            ("PriorityMin", i32::MIN, 2_i64, 5_i32),
        ];

        for (name, priority, minute_offset, index) in scenarios {
            enqueue(
                &pool,
                name,
                serde_json::json!({ "index": index }),
                base_time + chrono::Duration::minutes(minute_offset),
                None,
                None,
                Some(priority),
            )
            .await
            .expect("enqueue test job");
        }

        for expected_index in [1, 2, 3, 4, 5] {
            let job = dequeue(&pool, &[]).await.expect("dequeue failed");
            assert!(job.is_some());
            let job = job.unwrap();
            assert_eq!(
                job.data.get("index"),
                Some(&serde_json::json!(expected_index))
            );
            complete_job(&pool, &job.id, None)
                .await
                .expect("complete job");
        }

        let job = dequeue(&pool, &[]).await.expect("dequeue failed");
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn can_handle_worker_panic() {
        let (pool, _container) = setup_pg_test().await;

        let job_data: JobData = serde_json::json!(null);
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
        let handles = registry.run::<PgDriver>(&pool, &opts, &token, &[]);

        // Wait a bit for the worker to process the job
        sleep(Duration::from_secs(1)).await;

        // Stop the worker
        for handle in handles {
            handle.abort();
        }

        // Verify the job is marked as failed
        let failed_job = get_job(&pool, &job_id).await;
        assert_eq!(failed_job.status, JobStatus::Failed);

        // Verify the error message stored in job data
        let error_msg = failed_job
            .data
            .as_array()
            .and_then(|arr| arr.get(1))
            .and_then(|obj| obj.as_object())
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
        let (pool, _container) = setup_pg_test().await;

        // Add a job with email tag
        let run_at = Utc::now() - chrono::Duration::minutes(5); // In the past so it's ready to process
        let job_data = serde_json::json!({"user_id": 1});

        // Insert email job
        let email_tags = Some(vec!["email".to_string()]);
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
