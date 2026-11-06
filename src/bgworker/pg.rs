/// Postgres based background job queue provider
use std::time::Duration;

pub use super::sql::{Job, JobData, JobId, JobRegistry, RunOpts};
use super::{
    sql::{to_job, Driver},
    JobHandler, JobStatus, Queue, QueueProvider, RetryPolicy,
};
use crate::{
    config::{PostgresQueueConfig, RetentionConfig},
    Result,
};
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

    async fn record_attempt(pool: &Self::Pool, id: &JobId) -> crate::Result<u32> {
        let attempts: i32 = sqlx::query_scalar(
            "UPDATE pg_loco_queue SET attempts = attempts + 1 WHERE id = $1 RETURNING attempts",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(u32::try_from(attempts).unwrap_or(u32::MAX))
    }

    async fn retry_job(pool: &Self::Pool, id: &JobId, run_at: DateTime<Utc>) -> crate::Result<()> {
        requeue_unless_twin(
            pool,
            id,
            JobStatus::Processing,
            JobStatus::Cancelled,
            Some(run_at),
        )
        .await?;
        Ok(())
    }

    async fn heartbeat(pool: &Self::Pool, id: &JobId) -> crate::Result<()> {
        sqlx::query("UPDATE pg_loco_queue SET updated_at = NOW() WHERE id = $1 AND status = $2")
            .bind(id)
            .bind(JobStatus::Processing.to_string())
            .execute(pool)
            .await?;
        Ok(())
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

    async fn enqueue_unique(
        &self,
        class: String,
        _queue: Option<String>,
        args: serde_json::Value,
        tags: Option<Vec<String>>,
        priority: Option<i32>,
        key: String,
    ) -> Result<Option<String>> {
        Ok(Some(
            enqueue_unique(&self.pool, &class, args, tags, priority, &key).await?,
        ))
    }

    async fn register_handler(&self, name: String, handler: JobHandler) -> Result<()> {
        let mut registry = self.registry.lock().await;
        registry.insert_handler(name, handler)
    }

    async fn set_retry_policy(&self, name: String, policy: RetryPolicy) -> Result<()> {
        self.registry.lock().await.set_retry_policy(name, policy)
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
        if let Some(retention) = self.run_opts.retention.clone() {
            let pool = self.pool.clone();
            let token = self.token.clone();
            tokio::spawn(async move {
                let interval = std::time::Duration::from_secs(retention.interval_seconds);
                loop {
                    tokio::select! {
                        () = token.cancelled() => break,
                        () = tokio::time::sleep(interval) => {
                            if let Err(err) = apply_retention(&pool, &retention).await {
                                tracing::error!(error = %err, "retention: failed to delete finished jobs");
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

    // One transaction under an advisory lock: apps starting together queue up
    // here, and the ones that wait find the work done. Postgres DDL is
    // transactional, so a failure leaves nothing half-applied.
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(SETUP_LOCK_KEY)
        .execute(&mut *tx)
        .await?;

    // Check if the table already exists.
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_name = 'pg_loco_queue'
        )",
    )
    .fetch_one(&mut *tx)
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
        .fetch_one(&mut *tx)
        .await?;

        if !priority_exists {
            debug!("Adding priority column to existing pg_loco_queue table");
            sqlx::query("ALTER TABLE pg_loco_queue ADD COLUMN priority INT NOT NULL DEFAULT 0")
                .execute(&mut *tx)
                .await?;
        }
    } else {
        sqlx::raw_sql(AssertSqlSafe(format!(
            r"
                CREATE TABLE pg_loco_queue (
                    id VARCHAR NOT NULL PRIMARY KEY,
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
        .execute(&mut *tx)
        .await?;
    }

    // Tables created before the primary key existed.
    let has_primary_key: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT FROM pg_constraint
            WHERE conrelid = 'pg_loco_queue'::regclass AND contype = 'p'
        )",
    )
    .fetch_one(&mut *tx)
    .await?;
    if !has_primary_key {
        let duplicate: Option<String> = sqlx::query_scalar(
            "SELECT id FROM pg_loco_queue GROUP BY id HAVING COUNT(*) > 1 LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(id) = duplicate {
            return Err(crate::Error::string(&format!(
                "cannot add a primary key to pg_loco_queue: job id '{id}' appears more than \
                 once. Remove the duplicate rows (keep one per id) and start again"
            )));
        }
        debug!("Adding primary key to existing pg_loco_queue table");
        sqlx::query("ALTER TABLE pg_loco_queue ADD PRIMARY KEY (id)")
            .execute(&mut *tx)
            .await?;
    }

    // Columns and indexes added after the first release, on new and existing
    // tables alike.
    sqlx::raw_sql(AssertSqlSafe(format!(
        r"
            ALTER TABLE pg_loco_queue ADD COLUMN IF NOT EXISTS attempts INT NOT NULL DEFAULT 0;
            ALTER TABLE pg_loco_queue ADD COLUMN IF NOT EXISTS unique_key VARCHAR;
            CREATE UNIQUE INDEX IF NOT EXISTS pg_loco_queue_unique_queued
                ON pg_loco_queue (name, unique_key) WHERE {UNIQUE_QUEUED};
            CREATE INDEX IF NOT EXISTS pg_loco_queue_dequeue
                ON pg_loco_queue (priority DESC, run_at, id) WHERE status = 'queued';
            CREATE INDEX IF NOT EXISTS pg_loco_queue_status_updated_at
                ON pg_loco_queue (status, updated_at);
        "
    )))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Key of the advisory lock that serializes [`initialize_database`] across
/// processes.
const SETUP_LOCK_KEY: i64 = 0x6C6F_636F_7175_6575;

/// Predicate of the partial unique index behind `enqueue_unique`. The
/// `ON CONFLICT` target must repeat it verbatim.
const UNIQUE_QUEUED: &str = "status = 'queued' AND unique_key IS NOT NULL";

/// True when another job with this row's name and unique key is queued. The
/// unique index allows only one, so this row cannot become queued too.
const QUEUED_TWIN: &str = "unique_key IS NOT NULL AND EXISTS (SELECT 1 FROM pg_loco_queue AS twin \
                           WHERE twin.name = pg_loco_queue.name AND twin.unique_key = pg_loco_queue.unique_key \
                           AND twin.status = 'queued' AND twin.id <> pg_loco_queue.id)";

/// Moves job `id` from `from` back to queued. If a twin with the same unique
/// key is already queued, it moves to `on_twin` instead: the twin will do the
/// work, and requeueing would violate the unique index behind
/// `enqueue_unique`. `run_at: None` keeps the current `run_at`. Returns
/// whether the job was queued.
async fn requeue_unless_twin(
    pool: &PgPool,
    id: &str,
    from: JobStatus,
    on_twin: JobStatus,
    run_at: Option<DateTime<Utc>>,
) -> Result<bool> {
    let status: Option<String> = sqlx::query_scalar(AssertSqlSafe(format!(
        "UPDATE pg_loco_queue SET status = CASE WHEN {QUEUED_TWIN} THEN $1 ELSE $2 END, \
         updated_at = NOW(), run_at = COALESCE($3, run_at) WHERE id = $4 AND status = $5 \
         RETURNING status"
    )))
    .bind(on_twin.to_string())
    .bind(JobStatus::Queued.to_string())
    .bind(run_at)
    .bind(id)
    .bind(from.to_string())
    .fetch_optional(pool)
    .await?;
    let queued = status.as_deref() == Some(JobStatus::Queued.to_string().as_str());
    if status.is_some() && !queued {
        debug!(job_id = %id, "A twin with the same unique key is queued; not requeueing");
    }
    Ok(queued)
}

/// Add a job unless a queued job with the same `name` and `key` exists, and
/// return the ID of the queued job either way. See
/// [`Queue::enqueue_unique`](super::Queue::enqueue_unique).
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn enqueue_unique(
    pool: &PgPool,
    name: &str,
    data: JobData,
    tags: Option<Vec<String>>,
    priority: Option<i32>,
    key: &str,
) -> Result<JobId> {
    let tags_json = match &tags {
        Some(tags) => Some(serde_json::to_value(tags)?),
        None => None,
    };
    // A worker can take the existing job between the insert and the lookup,
    // leaving nothing queued under the key; the second insert then succeeds.
    for _ in 0..2 {
        let id = Ulid::new().to_string();
        let inserted: Option<String> = sqlx::query_scalar(AssertSqlSafe(format!(
            "INSERT INTO pg_loco_queue (id, task_data, name, run_at, tags, priority, unique_key) \
             VALUES ($1, $2, $3, NOW(), $4, $5, $6) ON CONFLICT (name, unique_key) WHERE \
             {UNIQUE_QUEUED} DO NOTHING RETURNING id"
        )))
        .bind(&id)
        .bind(&data)
        .bind(name)
        .bind(&tags_json)
        .bind(priority.unwrap_or(0))
        .bind(key)
        .fetch_optional(pool)
        .await?;
        if let Some(id) = inserted {
            debug!(job_id = %id, job_name = %name, key, "Enqueued unique job");
            return Ok(id);
        }
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM pg_loco_queue WHERE name = $1 AND unique_key = $2 AND status = $3",
        )
        .bind(name)
        .bind(key)
        .bind(JobStatus::Queued.to_string())
        .fetch_optional(pool)
        .await?;
        if let Some(id) = existing {
            debug!(job_id = %id, job_name = %name, key, "Unique job already queued");
            return Ok(id);
        }
    }
    Err(crate::Error::string(&format!(
        "could not enqueue unique job `{name}` with key `{key}`"
    )))
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

    // `status` is a literal, not a bind: a generic plan cannot prove a bound
    // value matches the partial index behind this query.
    let mut query = String::from(
        "SELECT id, name, task_data, status, run_at, interval, tags, priority FROM pg_loco_queue WHERE status = 'queued' AND run_at <= NOW() ",
    );

    // An untagged worker takes only untagged jobs; a tagged one takes any job
    // carrying at least one of its tags. `?` is jsonb's "array contains this
    // string" operator, one bind per tag starting at $1.
    if worker_tags.is_empty() {
        query.push_str("AND (tags IS NULL) ");
    } else {
        let any_tag_matches = (0..worker_tags.len())
            .map(|i| format!("(tags)::jsonb ? ${}", i + 1))
            .collect::<Vec<_>>()
            .join(" OR ");
        query.push_str("AND (tags IS NOT NULL) AND (");
        query.push_str(&any_tag_matches);
        query.push(')');
    }

    query.push_str(" ORDER BY priority DESC, run_at, id LIMIT 1 FOR UPDATE SKIP LOCKED");

    // Create the query
    let mut db_query = sqlx::query(AssertSqlSafe(query));

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

/// Deletes up to `batch` jobs in `status` that finished more than `age_days`
/// days ago, returning how many it deleted.
async fn delete_finished_batch(
    pool: &PgPool,
    status: &JobStatus,
    age_days: i64,
    batch: i64,
) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM pg_loco_queue WHERE id IN (
            SELECT id FROM pg_loco_queue
            WHERE status = $1 AND updated_at < NOW() - INTERVAL '1 day' * $2
            LIMIT $3
        )",
    )
    .bind(status.to_string())
    .bind(age_days)
    .bind(batch)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Runs one retention sweep: deletes finished jobs older than the ages in
/// `config`, a batch at a time. Never touches queued or processing jobs.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn apply_retention(pool: &PgPool, config: &RetentionConfig) -> Result<u64> {
    let deleted = super::sql::sweep_retention(config, |status, age_days, batch| async move {
        delete_finished_batch(pool, &status, age_days, batch).await
    })
    .await?;
    if deleted > 0 {
        debug!(deleted, "retention: deleted finished jobs");
    }
    Ok(deleted)
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
    let stale = format!("status = $1 AND updated_at <= NOW() - INTERVAL '{age_minutes} MINUTE'");

    debug!(age_minutes = age_minutes, "Requeueing stalled jobs");
    sqlx::query(AssertSqlSafe(format!(
        "UPDATE pg_loco_queue SET status = $2, updated_at = NOW() WHERE {stale} AND \
         unique_key IS NULL"
    )))
    .bind(JobStatus::Processing.to_string())
    .bind(JobStatus::Queued.to_string())
    .execute(pool)
    .await?;

    // One at a time, so of several stale twins only the first is requeued.
    let unique: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT id FROM pg_loco_queue WHERE {stale} AND unique_key IS NOT NULL ORDER BY id"
    )))
    .bind(JobStatus::Processing.to_string())
    .fetch_all(pool)
    .await?;
    for id in unique {
        requeue_unless_twin(pool, &id, JobStatus::Processing, JobStatus::Cancelled, None).await?;
    }

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
    // A failed job whose unique key already has a queued twin stays failed:
    // the twin does the same work, and the unique index allows one.
    let now = Some(Utc::now());
    if let Some(id) = id {
        debug!(job_id = %id, "Retrying failed job");
        let id: Option<String> =
            sqlx::query_scalar("SELECT id FROM pg_loco_queue WHERE status = $1 AND id::text = $2")
                .bind(JobStatus::Failed.to_string())
                .bind(id)
                .fetch_optional(pool)
                .await?;
        let Some(id) = id else { return Ok(0) };
        let queued =
            requeue_unless_twin(pool, &id, JobStatus::Failed, JobStatus::Failed, now).await?;
        return Ok(u64::from(queued));
    }

    debug!("Retrying every failed job");
    let mut moved = sqlx::query(
        "UPDATE pg_loco_queue SET status = $1, updated_at = NOW(), run_at = NOW() \
         WHERE status = $2 AND unique_key IS NULL",
    )
    .bind(JobStatus::Queued.to_string())
    .bind(JobStatus::Failed.to_string())
    .execute(pool)
    .await?
    .rows_affected();
    let unique: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM pg_loco_queue WHERE status = $1 AND unique_key IS NOT NULL ORDER BY id",
    )
    .bind(JobStatus::Failed.to_string())
    .fetch_all(pool)
    .await?;
    for id in unique {
        if requeue_unless_twin(pool, &id, JobStatus::Failed, JobStatus::Failed, now).await? {
            moved += 1;
        }
    }
    Ok(moved)
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
            retention: qcfg.retention.clone(),
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
        tests_cfg::{self, postgres::setup_postgres_database},
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
    async fn setup_pg_test() -> (PgPool, tests_cfg::postgres::TestPostgres) {
        let (pg_url, container) = setup_postgres_database().await;
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
    'pg_loco_queue' ORDER BY ordinal_position",
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
             ('job5', 'Test Job 5', '{}', 'processing', NOW(), NOW(), NOW())"
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
        let (pg_url, _container) = tests_cfg::postgres::setup_postgres_database().await;
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
            retention: None,
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
        let (pg_url, _container) = tests_cfg::postgres::setup_postgres_database().await;
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
            retention: None,
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
            retention: None,
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

    async fn attempts(pool: &PgPool, id: &str) -> i32 {
        sqlx::query_scalar("SELECT attempts FROM pg_loco_queue WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn enqueue_now(pool: &PgPool, name: &str) -> JobId {
        enqueue(
            pool,
            name,
            serde_json::json!(null),
            Utc::now(),
            None,
            None,
            None,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn enqueue_unique_returns_the_queued_job_for_a_repeated_key() {
        let (pool, _container) = setup_pg_test().await;
        let data = serde_json::json!({"account": 42});

        let first = enqueue_unique(&pool, "Sync", data.clone(), None, None, "account:42")
            .await
            .unwrap();
        let second = enqueue_unique(&pool, "Sync", data.clone(), None, None, "account:42")
            .await
            .unwrap();
        let other_key = enqueue_unique(&pool, "Sync", data.clone(), None, None, "account:7")
            .await
            .unwrap();
        let other_worker = enqueue_unique(&pool, "Audit", data, None, None, "account:42")
            .await
            .unwrap();

        assert_eq!(first, second);
        assert_ne!(first, other_key);
        assert_ne!(first, other_worker);
        assert_eq!(get_all_jobs(&pool).await.len(), 3);
    }

    #[tokio::test]
    async fn concurrent_enqueue_unique_calls_share_one_job() {
        let (pool, _container) = setup_pg_test().await;

        let calls = (0..20).map(|_| {
            let pool = pool.clone();
            tokio::spawn(async move {
                enqueue_unique(&pool, "Sync", serde_json::json!(null), None, None, "k").await
            })
        });
        let mut ids = Vec::new();
        for call in calls {
            ids.push(call.await.unwrap().unwrap());
        }

        ids.dedup();
        assert_eq!(ids.len(), 1);
        assert_eq!(get_all_jobs(&pool).await.len(), 1);
    }

    #[tokio::test]
    async fn enqueue_unique_adds_a_new_job_once_the_queued_one_is_running() {
        let (pool, _container) = setup_pg_test().await;
        let data = serde_json::json!(null);

        let first = enqueue_unique(&pool, "Sync", data.clone(), None, None, "k")
            .await
            .unwrap();
        let running = dequeue(&pool, &[]).await.unwrap().expect("job is due");
        assert_eq!(running.id, first);

        let second = enqueue_unique(&pool, "Sync", data.clone(), None, None, "k")
            .await
            .unwrap();
        assert_ne!(first, second);
        let third = enqueue_unique(&pool, "Sync", data, None, None, "k")
            .await
            .unwrap();
        assert_eq!(second, third);
    }

    async fn status_of(pool: &PgPool, id: &str) -> String {
        sqlx::query_scalar("SELECT status FROM pg_loco_queue WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn retry_and_reaper_cancel_a_unique_job_whose_twin_is_queued() {
        let (pool, _container) = setup_pg_test().await;
        let first = enqueue_unique(&pool, "Sync", serde_json::json!(null), None, None, "k")
            .await
            .unwrap();
        dequeue(&pool, &[]).await.unwrap().expect("job is due");
        let twin = enqueue_unique(&pool, "Sync", serde_json::json!(null), None, None, "k")
            .await
            .unwrap();
        PgDriver::retry_job(&pool, &first, Utc::now())
            .await
            .expect("retry must not hit the unique index");
        assert_eq!(status_of(&pool, &first).await, "cancelled");

        dequeue(&pool, &[]).await.unwrap().expect("twin is due");
        let third = enqueue_unique(&pool, "Sync", serde_json::json!(null), None, None, "k")
            .await
            .unwrap();
        sqlx::query("UPDATE pg_loco_queue SET updated_at = NOW() - INTERVAL '1 hour'")
            .execute(&pool)
            .await
            .unwrap();
        requeue(&pool, &1)
            .await
            .expect("a twin must not block the reaper");
        assert_eq!(status_of(&pool, &twin).await, "cancelled");
        assert_eq!(status_of(&pool, &third).await, "queued");
    }

    #[tokio::test]
    async fn plain_enqueue_is_never_deduplicated() {
        let (pool, _container) = setup_pg_test().await;
        enqueue_now(&pool, "Sync").await;
        enqueue_now(&pool, "Sync").await;
        assert_eq!(get_all_jobs(&pool).await.len(), 2);
    }

    #[tokio::test]
    async fn initialize_database_upgrades_a_table_from_before_retries() {
        let (pg_url, _container) = setup_postgres_database().await;
        let pool = PgPool::connect(&pg_url).await.unwrap();
        sqlx::query(
            "CREATE TABLE pg_loco_queue (id VARCHAR NOT NULL, name VARCHAR NOT NULL, task_data \
             JSONB NOT NULL, status VARCHAR NOT NULL DEFAULT 'queued', run_at TIMESTAMPTZ NOT \
             NULL, interval BIGINT, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at \
             TIMESTAMPTZ NOT NULL DEFAULT NOW(), tags JSONB, priority INT NOT NULL DEFAULT 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let old = enqueue_now(&pool, "Old").await;

        initialize_database(&pool).await.unwrap();
        initialize_database(&pool).await.unwrap();

        assert_eq!(attempts(&pool, &old).await, 0);
        let a = enqueue_unique(&pool, "Old", serde_json::json!(1), None, None, "k").await;
        let b = enqueue_unique(&pool, "Old", serde_json::json!(1), None, None, "k").await;
        assert_eq!(a.unwrap(), b.unwrap());
    }

    fn retrying(max_retries: u32, backoff: Duration) -> RetryPolicy {
        RetryPolicy {
            max_retries,
            backoff,
        }
    }

    /// Fails its first `fail_first` runs, then succeeds, counting every run.
    struct Flaky {
        fail_first: usize,
        runs: Arc<std::sync::atomic::AtomicUsize>,
    }

    fn flaky(fail_first: usize) -> (Flaky, Arc<std::sync::atomic::AtomicUsize>) {
        let runs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        (
            Flaky {
                fail_first,
                runs: runs.clone(),
            },
            runs,
        )
    }

    #[async_trait::async_trait]
    impl BackgroundWorker<()> for Flaky {
        fn build(_ctx: &crate::app::AppContext) -> Self {
            unreachable!()
        }
        async fn perform(&self, (): ()) -> crate::Result<()> {
            let run = self.runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if run < self.fail_first {
                Err(crate::Error::string("flaky failure"))
            } else {
                Ok(())
            }
        }
    }

    fn registry_with(worker: Flaky, policy: Option<RetryPolicy>) -> JobRegistry {
        let mut registry = JobRegistry::new();
        registry
            .insert_handler("Flaky".to_string(), crate::bgworker::erase_worker(worker))
            .unwrap();
        if let Some(policy) = policy {
            registry
                .set_retry_policy("Flaky".to_string(), policy)
                .unwrap();
        }
        registry
    }

    async fn run_for(registry: &JobRegistry, pool: &PgPool, wait: Duration) {
        let opts = RunOpts {
            num_workers: 1,
            poll_interval_sec: 1,
            reaper: None,
            retention: None,
        };
        let token = CancellationToken::new();
        let handles = registry.run::<PgDriver>(pool, &opts, &token, &[]);
        sleep(wait).await;
        for handle in handles {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn a_failing_job_runs_max_retries_more_times_then_fails() {
        let (pool, _container) = setup_pg_test().await;
        let id = enqueue_now(&pool, "Flaky").await;
        let (worker, runs) = flaky(usize::MAX);
        let registry = registry_with(worker, Some(retrying(2, Duration::ZERO)));

        run_for(&registry, &pool, Duration::from_millis(1500)).await;

        assert_eq!(runs.load(std::sync::atomic::Ordering::SeqCst), 3);
        assert_eq!(get_job(&pool, &id).await.status, JobStatus::Failed);
        assert_eq!(attempts(&pool, &id).await, 3);
    }

    #[tokio::test]
    async fn a_job_that_succeeds_on_retry_completes() {
        let (pool, _container) = setup_pg_test().await;
        let id = enqueue_now(&pool, "Flaky").await;
        let (worker, runs) = flaky(1);
        let registry = registry_with(worker, Some(retrying(3, Duration::ZERO)));

        run_for(&registry, &pool, Duration::from_millis(1500)).await;

        assert_eq!(runs.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(get_job(&pool, &id).await.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn a_retry_waits_for_its_backoff() {
        let (pool, _container) = setup_pg_test().await;
        let id = enqueue_now(&pool, "Flaky").await;
        let (worker, runs) = flaky(usize::MAX);
        let registry = registry_with(worker, Some(retrying(5, Duration::from_secs(3600))));

        run_for(&registry, &pool, Duration::from_millis(1500)).await;

        assert_eq!(runs.load(std::sync::atomic::Ordering::SeqCst), 1);
        let job = get_job(&pool, &id).await;
        assert_eq!(job.status, JobStatus::Queued);
        assert!(
            job.run_at > Utc::now() + chrono::Duration::minutes(50),
            "{}",
            job.run_at
        );
    }

    #[tokio::test]
    async fn without_a_retry_policy_the_first_failure_is_final() {
        let (pool, _container) = setup_pg_test().await;
        let id = enqueue_now(&pool, "Flaky").await;
        let (worker, runs) = flaky(usize::MAX);
        let registry = registry_with(worker, None);

        run_for(&registry, &pool, Duration::from_millis(1500)).await;

        assert_eq!(runs.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(get_job(&pool, &id).await.status, JobStatus::Failed);
    }

    async fn age_an_hour(pool: &PgPool) {
        sqlx::query("UPDATE pg_loco_queue SET updated_at = NOW() - INTERVAL '60 minutes'")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_running_job_heartbeats_so_the_reaper_skips_it() {
        let (pool, _container) = setup_pg_test().await;
        let id = enqueue_now(&pool, "Slow").await;
        dequeue(&pool, &[]).await.unwrap().expect("job is due");
        // Pretend the job was picked up an hour ago.
        age_an_hour(&pool).await;

        let result = crate::bgworker::sql::run_with_heartbeat::<PgDriver>(
            &pool,
            &id,
            Duration::from_millis(100),
            async {
                sleep(Duration::from_millis(350)).await;
                Ok(())
            },
        )
        .await;
        assert!(result.is_ok());

        requeue(&pool, &30).await.unwrap();
        assert_eq!(get_job(&pool, &id).await.status, JobStatus::Processing);
    }

    #[tokio::test]
    async fn the_reaper_requeues_a_job_without_heartbeats() {
        let (pool, _container) = setup_pg_test().await;
        let id = enqueue_now(&pool, "Slow").await;
        dequeue(&pool, &[]).await.unwrap().expect("job is due");
        age_an_hour(&pool).await;

        requeue(&pool, &30).await.unwrap();
        assert_eq!(get_job(&pool, &id).await.status, JobStatus::Queued);
    }

    const OLD_TABLE: &str = "CREATE TABLE pg_loco_queue (id VARCHAR NOT NULL, name VARCHAR NOT \
        NULL, task_data JSONB NOT NULL, status VARCHAR NOT NULL DEFAULT 'queued', run_at \
        TIMESTAMPTZ NOT NULL, interval BIGINT, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), \
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), tags JSONB, priority INT NOT NULL \
        DEFAULT 0)";

    async fn index_names(pool: &PgPool) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT indexname::text FROM pg_indexes WHERE tablename = 'pg_loco_queue' ORDER BY 1",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn has_primary_key(pool: &PgPool) -> bool {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM pg_constraint WHERE conrelid = 'pg_loco_queue'::regclass \
             AND contype = 'p')",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn setup_creates_the_primary_key_and_indexes_and_is_idempotent() {
        let (pool, _container) = setup_pg_test().await;
        initialize_database(&pool).await.unwrap();

        assert!(has_primary_key(&pool).await);
        let names = index_names(&pool).await;
        for expected in [
            "pg_loco_queue_dequeue",
            "pg_loco_queue_status_updated_at",
            "pg_loco_queue_unique_queued",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected}: {names:?}"
            );
        }
    }

    #[tokio::test]
    async fn setup_adds_the_primary_key_to_an_existing_table() {
        let (pg_url, _container) = setup_postgres_database().await;
        let pool = PgPool::connect(&pg_url).await.unwrap();
        sqlx::query(OLD_TABLE).execute(&pool).await.unwrap();
        let id = enqueue_now(&pool, "Old").await;
        assert!(!has_primary_key(&pool).await);

        initialize_database(&pool).await.unwrap();
        initialize_database(&pool).await.unwrap();

        assert!(has_primary_key(&pool).await);
        assert!(index_names(&pool)
            .await
            .contains(&"pg_loco_queue_dequeue".to_string()));
        assert_eq!(get_job(&pool, &id).await.name, "Old");
    }

    #[tokio::test]
    async fn setup_fails_clearly_on_duplicate_ids() {
        let (pg_url, _container) = setup_postgres_database().await;
        let pool = PgPool::connect(&pg_url).await.unwrap();
        sqlx::query(OLD_TABLE).execute(&pool).await.unwrap();
        for _ in 0..2 {
            sqlx::query(
                "INSERT INTO pg_loco_queue (id, name, task_data, run_at) VALUES ('dup', 'A', \
                 '{}', NOW())",
            )
            .execute(&pool)
            .await
            .unwrap();
        }

        let err = initialize_database(&pool).await.unwrap_err().to_string();
        assert!(err.contains("'dup' appears more than once"), "{err}");
        assert!(!has_primary_key(&pool).await);
    }

    #[tokio::test]
    async fn concurrent_setup_calls_all_succeed() {
        let (pg_url, _container) = setup_postgres_database().await;
        let pool = PgPool::connect(&pg_url).await.unwrap();
        let results =
            futures_util::future::join_all((0..4).map(|_| initialize_database(&pool))).await;
        for result in results {
            result.unwrap();
        }
        assert!(has_primary_key(&pool).await);
    }

    async fn insert_job(pool: &PgPool, id: &str, status: JobStatus, age_days: i64) {
        sqlx::query(
            "INSERT INTO pg_loco_queue (id, name, task_data, status, run_at, updated_at) VALUES \
             ($1, 'J', '{}', $2, NOW(), NOW() - INTERVAL '1 day' * $3)",
        )
        .bind(id)
        .bind(status.to_string())
        .bind(age_days)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn remaining_ids(pool: &PgPool) -> Vec<String> {
        sqlx::query_scalar("SELECT id FROM pg_loco_queue ORDER BY id")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn retention_deletes_only_old_finished_jobs_with_an_age() {
        let (pool, _container) = setup_pg_test().await;
        insert_job(&pool, "completed_old", JobStatus::Completed, 10).await;
        insert_job(&pool, "completed_new", JobStatus::Completed, 1).await;
        insert_job(&pool, "cancelled_old", JobStatus::Cancelled, 10).await;
        insert_job(&pool, "failed_old", JobStatus::Failed, 10).await;
        insert_job(&pool, "queued_old", JobStatus::Queued, 100).await;
        insert_job(&pool, "processing_old", JobStatus::Processing, 100).await;

        let config = RetentionConfig {
            completed_days: Some(7),
            cancelled_days: Some(7),
            failed_days: None,
            interval_seconds: 3600,
        };
        let deleted = apply_retention(&pool, &config).await.unwrap();

        assert_eq!(deleted, 2);
        assert_eq!(
            remaining_ids(&pool).await,
            [
                "completed_new",
                "failed_old",
                "processing_old",
                "queued_old"
            ]
        );
    }

    #[tokio::test]
    async fn retention_without_ages_deletes_nothing() {
        let (pool, _container) = setup_pg_test().await;
        insert_job(&pool, "completed_old", JobStatus::Completed, 1000).await;
        let deleted = apply_retention(&pool, &RetentionConfig::default())
            .await
            .unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(remaining_ids(&pool).await, ["completed_old"]);
    }

    #[tokio::test]
    async fn retention_loops_over_batches() {
        let (pool, _container) = setup_pg_test().await;
        sqlx::query(
            "INSERT INTO pg_loco_queue (id, name, task_data, status, run_at, updated_at) SELECT \
             'job' || g, 'J', '{}', 'completed', NOW(), NOW() - INTERVAL '30 days' FROM \
             generate_series(1, 2500) g",
        )
        .execute(&pool)
        .await
        .unwrap();
        let config = RetentionConfig {
            completed_days: Some(7),
            ..RetentionConfig::default()
        };
        assert_eq!(apply_retention(&pool, &config).await.unwrap(), 2500);
        assert!(remaining_ids(&pool).await.is_empty());
    }
}
