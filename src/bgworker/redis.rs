/// Redis based background job queue provider
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

pub use super::{Job, JobData, JobId};
use super::{JobHandler, JobStatus, Queue, QueueProvider};
use crate::{
    config::{ReaperConfig, RedisQueueConfig},
    Error, Result,
};
use async_trait::async_trait;
use chrono::Utc;
use redis::{aio::MultiplexedConnection as Connection, AsyncCommands, Client, Script};
use serde_json::Value as JsonValue;
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};
use ulid::Ulid;

pub type RedisPool = Client;

const QUEUE_KEY_PREFIX: &str = "queue:";
const JOB_KEY_PREFIX: &str = "job:";
const PROCESSING_KEY_PREFIX: &str = "processing:";

// Implementation for job creation and serialization
impl Job {
    fn new(id: String, name: String, data: JsonValue) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            data,
            status: JobStatus::Queued,
            run_at: now,
            interval: None,
            created_at: Some(now),
            updated_at: Some(now),
            tags: None,
            priority: 0,
        }
    }

    // Create JSON format for storing in Redis
    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    // Parse from JSON format
    fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

pub struct JobRegistry {
    handlers: Arc<HashMap<String, JobHandler>>,
}

impl JobRegistry {
    /// Creates a new [`JobRegistry`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(HashMap::new()),
        }
    }

    /// Inserts a pre-erased job handler under `name` (see
    /// [`super::erase_worker`]).
    ///
    /// # Errors
    ///
    /// Fails if cannot register worker
    pub fn insert_handler(&mut self, name: String, handler: JobHandler) -> Result<()> {
        Arc::get_mut(&mut self.handlers)
            .ok_or_else(|| Error::string("cannot register worker"))?
            .insert(name, handler);
        Ok(())
    }

    /// Returns a reference to the job handlers.
    #[must_use]
    pub fn handlers(&self) -> &Arc<HashMap<String, JobHandler>> {
        &self.handlers
    }

    /// Runs the job handlers with the provided number of workers.
    #[must_use]
    pub fn run(
        &self,
        client: &RedisPool,
        opts: &RunOpts,
        token: &CancellationToken,
        tags: &[String],
    ) -> Vec<JoinHandle<()>> {
        let mut jobs = Vec::new();
        let queues = get_queues(&opts.queues);
        let interval = opts.poll_interval_sec;

        for idx in 0..opts.num_workers {
            let handlers = self.handlers.clone();
            let worker_token = token.clone();
            let client = client.clone();
            let queues = queues.clone();
            let tags = tags.to_owned();

            let job = tokio::spawn(async move {
                let mut conn = match client.get_multiplexed_async_connection().await {
                    Ok(conn) => conn,
                    Err(err) => {
                        error!(err = err.to_string(), "Failed to create worker connection");
                        return;
                    }
                };

                loop {
                    // Check for cancellation before potentially blocking on dequeue
                    if worker_token.is_cancelled() {
                        trace!(worker_num = idx, "cancellation received, stopping worker");
                        break;
                    }

                    let job_opt = match dequeue_with_conn(&mut conn, &queues, &tags).await {
                        Ok(t) => t,
                        Err(err) => {
                            error!(err = err.to_string(), "cannot fetch from queue");
                            None
                        }
                    };

                    if let Some((job, queue_name)) = job_opt {
                        debug!(job_id = job.id, name = job.name, "working on job");
                        if let Some(handler) = handlers.get(&job.name) {
                            match handler(job.id.clone(), job.data.clone()).await {
                                Ok(()) => {
                                    if let Err(err) = complete_job_with_conn(
                                        &mut conn,
                                        &job.id,
                                        &queue_name,
                                        job.interval,
                                    )
                                    .await
                                    {
                                        error!(err = err.to_string(), job = ?job, "cannot complete job");
                                    }
                                }
                                Err(err) => {
                                    if let Err(err) =
                                        fail_job_with_conn(&mut conn, &job.id, &queue_name, &err)
                                            .await
                                    {
                                        error!(err = err.to_string(), job = ?job, "cannot fail job");
                                    }
                                }
                            }
                        } else {
                            error!(job = job.name, "no handler found for job");
                        }
                    } else {
                        tokio::select! {
                            biased;
                            () = worker_token.cancelled() => {
                                trace!(worker_num = idx, "cancellation received during sleep, stopping worker");
                                break;
                            }
                            () = sleep(Duration::from_secs(interval.into())) => {}
                        }
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

fn connect(url: &str) -> Result<RedisPool> {
    let client = Client::open(url.to_string())?;
    Ok(client)
}

async fn get_connection(client: &RedisPool) -> Result<Connection> {
    let conn = client.get_multiplexed_async_connection().await?;
    Ok(conn)
}

/// Clear tasks
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear(client: &RedisPool) -> Result<()> {
    let mut conn = get_connection(client).await?;
    redis::cmd("FLUSHDB").query_async::<()>(&mut conn).await?;
    Ok(())
}

/// Add a task
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn enqueue(
    client: &RedisPool,
    class: String,
    queue: Option<String>,
    args: impl serde::Serialize + Send,
    tags: Option<Vec<String>>,
    priority: Option<i32>,
) -> Result<JobId> {
    let mut conn = get_connection(client).await?;
    let queue_name = queue.unwrap_or_else(|| "default".to_string());
    let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");

    // Convert args to JSON
    let args_json = serde_json::to_value(args)?;

    // Create a job ID using ULID
    let job_id = Ulid::new().to_string();

    // Create job
    let mut job = Job::new(job_id.clone(), class, args_json);
    job.tags = tags;
    job.priority = priority.unwrap_or(0);

    // Serialize job for Redis storage
    let job_json = job.to_json()?;

    // Store job in Redis queue (ZSET, scored by priority) and in its job key.
    let score = calculate_score(job.priority);
    let job_key = format!("{JOB_KEY_PREFIX}{}", job.id);
    let _: () = conn.set(&job_key, &job_json).await?;
    let _: () = conn.zadd(&queue_key, &job.id, score).await?;

    Ok(job_id)
}

/// Enqueue multiple jobs in a single atomic pipeline operation.
///
/// Each entry of `jobs` is one job's arguments paired with its priority
/// (`None` for the default); `tags` apply to every job. The returned IDs are
/// in the same order as `jobs`.
///
/// The pipeline runs as a `MULTI`/`EXEC` transaction, so either every job's
/// payload and queue entry are written or none are. A failure leaves nothing
/// behind and the batch is safe to retry without duplicating jobs.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn enqueue_batch(
    client: &RedisPool,
    class: String,
    queue: Option<String>,
    jobs: Vec<(serde_json::Value, Option<i32>)>,
    tags: Option<Vec<String>>,
) -> Result<Vec<JobId>> {
    if jobs.is_empty() {
        return Ok(Vec::new());
    }

    let mut conn = get_connection(client).await?;
    let queue_name = queue.unwrap_or_else(|| "default".to_string());
    let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");

    let mut ids = Vec::with_capacity(jobs.len());
    let mut pipe = redis::pipe();
    pipe.atomic();
    for (args_json, priority) in jobs {
        let job_id = Ulid::new().to_string();
        let mut job = Job::new(job_id, class.clone(), args_json);
        job.tags = tags.clone();
        job.priority = priority.unwrap_or(0);
        let job_json = job.to_json()?;
        let job_key = format!("{JOB_KEY_PREFIX}{}", job.id);
        pipe.set(&job_key, &job_json).ignore();
        pipe.zadd(&queue_key, &job.id, calculate_score(job.priority))
            .ignore();
        ids.push(job.id);
    }

    pipe.query_async::<()>(&mut conn).await?;
    Ok(ids)
}

/// Redis ZSET score for a job, derived from priority only.
///
/// We deliberately use only the priority in the score to preserve exact
/// ordering across the full `i32` range (a combined priority+timestamp score
/// would lose precision). The score is negated so that a plain ascending
/// `ZRANGE` yields the highest-priority jobs first. Timestamp/id tie-breaking
/// for equal priorities is handled explicitly in `dequeue_with_conn`.
fn calculate_score(priority: i32) -> f64 {
    -f64::from(priority)
}

const ACQUIRE_JOB_SCRIPT: &str = r"
local queue_key = KEYS[1]
local processing_key = KEYS[2]
local job_id = ARGV[1]

local score = redis.call('ZSCORE', queue_key, job_id)
if score then
    redis.call('ZREM', queue_key, job_id)
    redis.call('SADD', processing_key, job_id)
    return score
else
    return nil
end
";

async fn dequeue_with_conn(
    conn: &mut Connection,
    queues: &[String],
    tags: &[String],
) -> Result<Option<(Job, String)>> {
    // Paging bounds for scanning the priority-ordered queue.
    const BATCH_SIZE: isize = 50;
    const MAX_SEARCH: isize = 1000;

    if queues.is_empty() {
        return Ok(None);
    }

    let script = Script::new(ACQUIRE_JOB_SCRIPT);

    for queue_name in queues {
        let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");
        let processing_key = format!("{PROCESSING_KEY_PREFIX}{queue_name}");

        // Page through the queue (ordered by ZSET score = priority) collecting
        // jobs whose tags match the worker's tag filter.
        let mut offset = 0;
        let mut candidates: Vec<(String, Job)> = Vec::new();
        while offset < MAX_SEARCH {
            let job_ids: Vec<String> = conn
                .zrange(&queue_key, offset, offset + BATCH_SIZE - 1)
                .await?;
            if job_ids.is_empty() {
                break;
            }

            // Batch-fetch job data to minimize round trips.
            let mut pipe = redis::pipe();
            for job_id in &job_ids {
                pipe.get(format!("{JOB_KEY_PREFIX}{job_id}"));
            }
            let job_jsons: Vec<Option<String>> = pipe.query_async(conn).await?;

            for (job_id, job_json_opt) in job_ids.iter().zip(job_jsons) {
                if let Some(json) = job_json_opt {
                    match Job::from_json(&json) {
                        Ok(job) => {
                            let should_process = if tags.is_empty() {
                                job.tags.is_none() || job.tags.as_ref().is_none_or(Vec::is_empty)
                            } else {
                                job.tags.as_ref().is_some_and(|job_tags| {
                                    job_tags.iter().any(|tag| tags.contains(tag))
                                })
                            };

                            if !should_process {
                                trace!(
                                    job_id = job_id,
                                    job_tags = ?job.tags,
                                    worker_tags = ?tags,
                                    "Job doesn't match tag criteria, skipping"
                                );
                            } else if job.run_at > Utc::now() {
                                // Not yet due (e.g. an interval-rescheduled job):
                                // mirror the SQL backends' `run_at <= NOW()` filter
                                // so it isn't re-run before its scheduled time.
                                trace!(
                                    job_id = job_id,
                                    run_at = ?job.run_at,
                                    "Job not due yet, skipping"
                                );
                            } else {
                                candidates.push((job_id.clone(), job));
                            }
                        }
                        Err(err) => {
                            error!(
                                err = err.to_string(),
                                job_id = job_id,
                                "Failed to parse job JSON"
                            );
                            // Skip corrupted jobs during the scan; don't remove
                            // them here, to avoid data loss on transient issues.
                        }
                    }
                } else {
                    error!(job_id = job_id, queue = queue_name, "Job data not found.");
                    // Job ID exists in the queue but its data is gone: clean up.
                    let _: () = conn.zrem(&queue_key, job_id).await?;
                }
            }
            offset += BATCH_SIZE;
        }

        // Deterministic ordering:
        // 1. Higher priority first.
        // 2. Earlier `run_at` first for equal priority.
        // 3. Smaller id first as a final deterministic tiebreaker.
        candidates.sort_by(|(id_a, job_a), (id_b, job_b)| {
            job_b
                .priority
                .cmp(&job_a.priority)
                .then_with(|| job_a.run_at.cmp(&job_b.run_at))
                .then_with(|| id_a.cmp(id_b))
        });

        for (job_id, job) in candidates {
            // Atomically claim the job: move it from the queue ZSET to the
            // processing set. Returns None if another worker took it first.
            let result: Option<f64> = script
                .key(&queue_key)
                .key(&processing_key)
                .arg(&job_id)
                .invoke_async(conn)
                .await?;

            if result.is_some() {
                return Ok(Some((job, queue_name.clone())));
            }
        }
    }
    Ok(None)
}

async fn complete_job_with_conn(
    conn: &mut Connection,
    id: &JobId,
    queue_name: &str,
    interval_ms: Option<i64>,
) -> Result<()> {
    let job_key = format!("{JOB_KEY_PREFIX}{id}");
    let processing_key = format!("{PROCESSING_KEY_PREFIX}{queue_name}");

    let job_json: Option<String> = conn.get(&job_key).await?;
    if let Some(json) = job_json
        && let Ok(mut job) = Job::from_json(&json)
    {
        if let Some(interval) = interval_ms {
            job.run_at = Utc::now() + chrono::Duration::milliseconds(interval);
            job.status = JobStatus::Queued;
            let new_json = job.to_json()?;
            let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");
            let score = calculate_score(job.priority);
            let _: () = redis::pipe()
                .set(&job_key, &new_json)
                .zadd(&queue_key, id, score)
                .query_async(conn)
                .await?;
        } else {
            job.status = JobStatus::Completed;
            job.updated_at = Some(Utc::now());
            let updated_json = job.to_json()?;
            let _: () = conn.set(&job_key, &updated_json).await?;
        }
        let _: () = conn.srem(&processing_key, id).await?;
    }
    Ok(())
}

async fn fail_job_with_conn(
    conn: &mut Connection,
    id: &JobId,
    queue_name: &str,
    error: &crate::Error,
) -> Result<()> {
    let job_key = format!("{JOB_KEY_PREFIX}{id}");
    let processing_key = format!("{PROCESSING_KEY_PREFIX}{queue_name}");

    let job_json: Option<String> = conn.get(&job_key).await?;
    if let Some(json) = job_json
        && let Ok(mut job) = Job::from_json(&json)
    {
        // Preserve the original task arguments and attach the error alongside
        // them, mirroring the SQL backends (`task_data = task_data || {error}`)
        // instead of overwriting the args with the error payload.
        if let Some(obj) = job.data.as_object_mut() {
            obj.insert(
                "error".to_string(),
                serde_json::Value::String(error.to_string()),
            );
        } else {
            job.data = serde_json::json!({ "args": job.data, "error": error.to_string() });
        }
        job.status = JobStatus::Failed;
        job.updated_at = Some(Utc::now());
        let updated_json = job.to_json()?;
        let _: () = conn.set(&job_key, &updated_json).await?;
    }
    let _: () = conn.srem(&processing_key, id).await?;
    Ok(())
}

/// Ping system
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn ping(client: &RedisPool) -> Result<()> {
    let mut conn = get_connection(client).await?;
    let _: String = redis::cmd("PING").query_async(&mut conn).await?;
    Ok(())
}

/// Retrieves a list of jobs, optionally filtered by `status` and age.
///
/// Enumerates the `job:*` keys, which are the record of a job's existence, and
/// consults the processing sets only to distinguish a job a worker is holding
/// from one still waiting in a queue.
///
/// It deliberately does **not** enumerate the queues instead. A queue ZSET and
/// a processing set are scheduling structures: `complete_job` and `fail_job`
/// both remove the id from the processing set and add it to no queue, so a
/// job walked-to through those structures becomes invisible the instant it
/// stops being runnable — which is exactly when an operator goes looking for
/// it. Every job-listing tool is built on this function (`jobs dump`,
/// `jobs purge`, `clear_by_status`, `clear_jobs_older_than`), so all of them
/// silently reported nothing for completed and failed jobs.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn get_jobs(
    client: &RedisPool,
    status: Option<&Vec<JobStatus>>,
    age_days: Option<i64>,
) -> Result<Vec<Job>> {
    let mut conn = get_connection(client).await?;

    let processing_pattern = format!("{PROCESSING_KEY_PREFIX}*");
    let processing_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&processing_pattern)
        .query_async(&mut conn)
        .await?;

    let mut in_progress: HashSet<String> = HashSet::new();
    for processing_key in processing_keys {
        let job_ids: Vec<String> = conn.smembers(&processing_key).await?;
        in_progress.extend(job_ids);
    }

    let job_pattern = format!("{JOB_KEY_PREFIX}*");
    let job_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&job_pattern)
        .query_async(&mut conn)
        .await?;

    let mut jobs = Vec::new();
    for job_key in job_keys {
        let job_json: Option<String> = conn.get(&job_key).await?;
        let Some(json) = job_json else { continue };
        let Ok(mut job) = Job::from_json(&json) else {
            continue;
        };

        // A job a worker is holding is stored as `queued` — the status is only
        // rewritten when it finishes — so the processing set is what makes it
        // `processing`.
        if job.status == JobStatus::Queued && in_progress.contains(&job.id) {
            job.status = JobStatus::Processing;
        }

        if should_include_job(&job, status, age_days) {
            jobs.push(job);
        }
    }

    Ok(jobs)
}

// Helper function to check if a job matches the filter criteria
fn should_include_job(job: &Job, status: Option<&Vec<JobStatus>>, age_days: Option<i64>) -> bool {
    if let Some(status_list) = status
        && !status_list.contains(&job.status)
    {
        return false;
    }
    if let Some(age_days) = age_days
        && let Some(created_at) = job.created_at
    {
        let cutoff_date = Utc::now() - chrono::Duration::days(age_days);
        if created_at > cutoff_date {
            return false;
        }
    }
    true
}

/// Clears jobs based on their status from the Redis queue.
///
/// This function removes all jobs with a status matching any of the statuses provided
/// in the `status` argument. It searches through all queue keys and processing sets
/// and removes matching jobs.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear_by_status(client: &RedisPool, status: Vec<JobStatus>) -> Result<()> {
    let mut conn = get_connection(client).await?;

    // Get all queue keys
    let queue_pattern = format!("{QUEUE_KEY_PREFIX}*");
    let queue_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&queue_pattern)
        .query_async(&mut conn)
        .await?;

    // Get all processing keys
    let processing_pattern = format!("{PROCESSING_KEY_PREFIX}*");
    let processing_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&processing_pattern)
        .query_async(&mut conn)
        .await?;

    // Get all job keys
    let job_pattern = format!("{JOB_KEY_PREFIX}*");
    let job_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&job_pattern)
        .query_async(&mut conn)
        .await?;

    // Process queues
    for queue_key in queue_keys {
        // Get all jobs in the queue
        let job_ids: Vec<String> = conn.zrange(&queue_key, 0, -1).await?;

        // Process each job individually
        for job_id in job_ids {
            let job_key = format!("{JOB_KEY_PREFIX}{job_id}");
            let job_json: Option<String> = conn.get(&job_key).await?;
            if let Some(json) = job_json
                && let Ok(job) = Job::from_json(&json)
                && status.contains(&job.status)
            {
                let _: () = conn.zrem(&queue_key, &job_id).await?;
                let _: () = conn.del(&job_key).await?;
            }
        }
    }

    for processing_key in processing_keys {
        let job_ids: Vec<String> = conn.smembers(&processing_key).await?;
        for job_id in job_ids {
            let job_key = format!("{JOB_KEY_PREFIX}{job_id}");
            let job_json: Option<String> = conn.get(&job_key).await?;
            if let Some(json) = job_json
                && let Ok(mut job) = Job::from_json(&json)
            {
                if job.status == JobStatus::Queued {
                    job.status = JobStatus::Processing;
                }
                if status.contains(&job.status) {
                    let _: () = conn.srem(&processing_key, &job_id).await?;
                    let _: () = conn.del(&job_key).await?;
                }
            }
        }
    }

    for job_key in job_keys {
        let job_json: Option<String> = conn.get(&job_key).await?;
        if let Some(json) = job_json
            && let Ok(job) = Job::from_json(&json)
            && status.contains(&job.status)
        {
            let _: () = conn.del(&job_key).await?;
        }
    }

    Ok(())
}

/// Clears jobs older than the specified number of days from the Redis queue.
///
/// This function removes all jobs that were created more than `age_days` days ago
/// and have a status matching any of the statuses provided in the `status` argument.
/// It searches through all queue keys and processing sets and removes matching jobs.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear_jobs_older_than(
    client: &RedisPool,
    age_days: i64,
    status: Option<&Vec<JobStatus>>,
) -> Result<()> {
    let mut conn = get_connection(client).await?;
    let cutoff_date = Utc::now() - chrono::Duration::days(age_days);

    // Get all queue keys
    let queue_pattern = format!("{QUEUE_KEY_PREFIX}*");
    let queue_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&queue_pattern)
        .query_async(&mut conn)
        .await?;

    // Get all processing keys
    let processing_pattern = format!("{PROCESSING_KEY_PREFIX}*");
    let processing_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&processing_pattern)
        .query_async(&mut conn)
        .await?;

    // Get all job keys
    let job_pattern = format!("{JOB_KEY_PREFIX}*");
    let job_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&job_pattern)
        .query_async(&mut conn)
        .await?;

    // Process queues
    for queue_key in queue_keys {
        // Get all jobs in the queue
        let job_ids: Vec<String> = conn.zrange(&queue_key, 0, -1).await?;

        // Process each job individually
        for job_id in job_ids {
            let job_key = format!("{JOB_KEY_PREFIX}{job_id}");
            let job_json: Option<String> = conn.get(&job_key).await?;
            if let Some(json) = job_json
                && let Ok(job) = Job::from_json(&json)
            {
                let should_remove = job.created_at.is_some_and(|created_at| {
                    created_at < cutoff_date && status.is_none_or(|s| s.contains(&job.status))
                });
                if should_remove {
                    let _: () = conn.zrem(&queue_key, &job_id).await?;
                    let _: () = conn.del(&job_key).await?;
                }
            }
        }
    }

    for processing_key in processing_keys {
        let job_ids: Vec<String> = conn.smembers(&processing_key).await?;
        for job_id in job_ids {
            let job_key = format!("{JOB_KEY_PREFIX}{job_id}");
            let job_json: Option<String> = conn.get(&job_key).await?;
            if let Some(json) = job_json
                && let Ok(mut job) = Job::from_json(&json)
            {
                if job.status == JobStatus::Queued {
                    job.status = JobStatus::Processing;
                }
                let should_remove = job.created_at.is_some_and(|created_at| {
                    created_at < cutoff_date && status.is_none_or(|s| s.contains(&job.status))
                });
                if should_remove {
                    let _: () = conn.srem(&processing_key, &job_id).await?;
                    let _: () = conn.del(&job_key).await?;
                }
            }
        }
    }

    for job_key in job_keys {
        let job_json: Option<String> = conn.get(&job_key).await?;
        if let Some(json) = job_json
            && let Ok(job) = Job::from_json(&json)
        {
            let should_remove = job.created_at.is_some_and(|created_at| {
                created_at < cutoff_date && status.is_none_or(|s| s.contains(&job.status))
            });
            if should_remove {
                let _: () = conn.del(&job_key).await?;
            }
        }
    }

    Ok(())
}

/// Moves failed jobs back onto their queue, returning how many moved.
///
/// A failed job holds its args and its `error` under its `job:` key but sits
/// in no queue and no processing set, so retrying it means writing the status
/// back and re-adding the id to a queue ZSET.
///
/// It goes to `default`. The queue a job was submitted to is not recoverable:
/// `Job` carries no queue field, and the only record of the association — the
/// id's membership in that queue's ZSET — is what `fail_job` removed. Retrying
/// onto `default` is therefore a real behaviour change for a multi-queue setup,
/// and the CLI says so.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn retry_failed(client: &RedisPool, id: Option<&str>) -> Result<u64> {
    let mut conn = get_connection(client).await?;
    let failed = get_jobs(client, Some(&vec![JobStatus::Failed]), None).await?;

    let mut retried = 0;
    for mut job in failed {
        if let Some(id) = id
            && job.id != id
        {
            continue;
        }

        job.status = JobStatus::Queued;
        job.run_at = Utc::now();
        job.updated_at = Some(Utc::now());

        let job_key = format!("{JOB_KEY_PREFIX}{}", job.id);
        let queue_key = format!("{QUEUE_KEY_PREFIX}default");
        let _: () = conn.set(&job_key, job.to_json()?).await?;
        let _: () = conn
            .zadd(&queue_key, &job.id, calculate_score(job.priority))
            .await?;
        retried += 1;
    }

    debug!(retried = retried, "Retried failed jobs");
    Ok(retried)
}

/// Requeues failed or stalled jobs that are older than a specified number of minutes.
///
/// This function finds jobs in processing sets that have been there for longer than
/// `age_minutes` and moves them back to their respective queues. This is useful for
/// recovering from job failures or worker crashes.
///
/// # Errors
///
/// This function will return an error if it fails to interact with Redis
pub async fn requeue(client: &RedisPool, age_minutes: &i64) -> Result<()> {
    let mut conn = get_connection(client).await?;
    let cutoff_time = Utc::now() - chrono::Duration::minutes(*age_minutes);
    let mut requeued_counts: HashMap<String, usize> = HashMap::new();

    // Get all processing set keys
    let processing_pattern = format!("{PROCESSING_KEY_PREFIX}*");
    let processing_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&processing_pattern)
        .query_async(&mut conn)
        .await?;

    // Process each processing set
    for processing_key in processing_keys {
        // Extract queue name from processing key
        let queue_name = processing_key
            .trim_start_matches(PROCESSING_KEY_PREFIX)
            .to_string();
        let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");

        // Get all jobs in the processing set
        let job_ids: Vec<String> = conn.smembers(&processing_key).await?;

        // Check each job in the processing set
        for job_id in job_ids {
            let job_key = format!("{JOB_KEY_PREFIX}{job_id}");
            let job_json: Option<String> = conn.get(&job_key).await?;
            if let Some(json) = job_json
                && let Ok(mut job) = Job::from_json(&json)
            {
                let should_requeue = if let Some(updated_at) = job.updated_at {
                    updated_at < cutoff_time
                } else if let Some(created_at) = job.created_at {
                    created_at < cutoff_time
                } else {
                    false
                };
                if should_requeue {
                    job.status = JobStatus::Queued;
                    job.updated_at = Some(Utc::now());
                    let updated_json = job.to_json()?;
                    let score = calculate_score(job.priority);
                    let _: () = conn.srem(&processing_key, &job_id).await?;
                    let _: () = conn.set(&job_key, &updated_json).await?;
                    let _: () = conn.zadd(&queue_key, &job_id, score).await?;
                    *requeued_counts.entry(queue_name.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    let failed_pattern = "failed:*";
    let failed_keys: Vec<String> = redis::cmd("KEYS")
        .arg(failed_pattern)
        .query_async(&mut conn)
        .await?;

    for failed_key in failed_keys {
        let queue_name = failed_key.trim_start_matches("failed:").to_string();
        let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");
        let job_ids: Vec<String> = conn.smembers(&failed_key).await?;

        for job_id in job_ids {
            let job_key = format!("{JOB_KEY_PREFIX}{job_id}");
            let job_json: Option<String> = conn.get(&job_key).await?;
            if let Some(json) = job_json
                && let Ok(mut job) = Job::from_json(&json)
            {
                let should_requeue = if let Some(updated_at) = job.updated_at {
                    updated_at < cutoff_time && job.status == JobStatus::Failed
                } else {
                    false
                };
                if should_requeue {
                    job.status = JobStatus::Queued;
                    job.updated_at = Some(Utc::now());
                    let updated_json = job.to_json()?;
                    let score = calculate_score(job.priority);
                    let _: () = conn.srem(&failed_key, &job_id).await?;
                    let _: () = conn.set(&job_key, &updated_json).await?;
                    let _: () = conn.zadd(&queue_key, &job_id, score).await?;
                    *requeued_counts.entry(queue_name.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    for (queue, count) in requeued_counts {
        if count > 0 {
            debug!(queue = queue, count = count, "requeued jobs");
        }
    }
    Ok(())
}

/// Cancels jobs with the specified name in the Redis queue.
///
/// This function updates the status of jobs that match the provided `job_name`
/// from [`JobStatus::Queued`] to [`JobStatus::Cancelled`]. Jobs are searched for in all queue keys,
/// and only those that are currently in the [`JobStatus::Queued`] state will be affected.
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn cancel_jobs_by_name(client: &RedisPool, job_name: &str) -> Result<()> {
    let mut conn = get_connection(client).await?;

    // Get all queue keys
    let queue_pattern = format!("{QUEUE_KEY_PREFIX}*");
    let queue_keys: Vec<String> = redis::cmd("KEYS")
        .arg(&queue_pattern)
        .query_async(&mut conn)
        .await?;

    // Process each queue
    for queue_key in queue_keys {
        // Get all jobs in the queue
        let job_ids: Vec<String> = conn.zrange(&queue_key, 0, -1).await?;
        for job_id in job_ids {
            let job_key = format!("{JOB_KEY_PREFIX}{job_id}");
            let job_json: Option<String> = conn.get(&job_key).await?;
            if let Some(json) = job_json
                && let Ok(mut job) = Job::from_json(&json)
                && job.name == job_name
                && job.status == JobStatus::Queued
            {
                job.status = JobStatus::Cancelled;
                job.updated_at = Some(Utc::now());
                let updated_json = job.to_json()?;
                let _: () = conn.zrem(&queue_key, &job_id).await?;
                let _: () = conn.set(&job_key, &updated_json).await?;
                let cancelled_key = format!(
                    "cancelled:{}",
                    queue_key.trim_start_matches(QUEUE_KEY_PREFIX)
                );
                let _: () = conn.sadd(&cancelled_key, &job_id).await?;
            }
        }
    }
    Ok(())
}

pub const DEFAULT_QUEUES: &[&str] = &["default", "mailer"];

pub fn get_queues(config_queues: &Option<Vec<String>>) -> Vec<String> {
    let mut queues = DEFAULT_QUEUES
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if let Some(config_queues) = config_queues {
        for q in config_queues {
            if !queues.iter().any(|aq| q == aq) {
                queues.push(q.clone());
            }
        }
    }
    queues
}

pub struct RunOpts {
    pub num_workers: u32,
    pub poll_interval_sec: u32,
    pub queues: Option<Vec<String>>,
    /// Opt-in visibility-timeout reaper settings, populated from the queue
    /// config. `None` disables the reaper (default, backward-compatible).
    pub reaper: Option<ReaperConfig>,
}

/// Redis [`QueueProvider`]: holds the client, job registry, run options and
/// cancellation token that used to live in the `Queue::Redis(..)` enum
/// tuple.
pub struct RedisQueue {
    pub client: RedisPool,
    pub registry: Arc<tokio::sync::Mutex<JobRegistry>>,
    pub run_opts: RunOpts,
    pub token: CancellationToken,
}

#[async_trait]
impl QueueProvider for RedisQueue {
    async fn enqueue(
        &self,
        class: String,
        queue: Option<String>,
        args: JsonValue,
        tags: Option<Vec<String>>,
        priority: Option<i32>,
    ) -> Result<Option<String>> {
        Ok(Some(
            enqueue(&self.client, class, queue, args, tags, priority).await?,
        ))
    }

    async fn enqueue_batch(
        &self,
        class: String,
        queue: Option<String>,
        jobs: Vec<(JsonValue, Option<i32>)>,
        tags: Option<Vec<String>>,
    ) -> Result<Vec<JobId>> {
        enqueue_batch(&self.client, class, queue, jobs, tags).await
    }

    async fn register_handler(&self, name: String, handler: JobHandler) -> Result<()> {
        let mut registry = self.registry.lock().await;
        registry.insert_handler(name, handler)
    }

    async fn run(&self, tags: Vec<String>) -> Result<()> {
        if let Some(reaper) = self.run_opts.reaper.clone() {
            let pool = self.client.clone();
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
        let handles = self.registry.lock().await.run(
            &self.client,
            &self.run_opts,
            &self.token.clone(),
            &tags,
        );
        super::process_worker_handles(handles).await
    }

    async fn setup(&self) -> Result<()> {
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        clear(&self.client).await
    }

    async fn ping(&self) -> Result<()> {
        ping(&self.client).await
    }

    async fn get_jobs(
        &self,
        status: Option<&Vec<JobStatus>>,
        age_days: Option<i64>,
    ) -> Result<Vec<Job>> {
        get_jobs(&self.client, status, age_days).await
    }

    async fn cancel_jobs_by_name(&self, name: &str) -> Result<()> {
        cancel_jobs_by_name(&self.client, name).await
    }

    async fn clear_by_status(&self, status: Vec<JobStatus>) -> Result<()> {
        clear_by_status(&self.client, status).await
    }

    async fn clear_jobs_older_than(
        &self,
        age_days: i64,
        status: Option<&Vec<JobStatus>>,
    ) -> Result<()> {
        clear_jobs_older_than(&self.client, age_days, status).await
    }

    async fn retry_failed(&self, id: Option<&str>) -> Result<u64> {
        retry_failed(&self.client, id).await
    }

    async fn requeue(&self, age_minutes: &i64) -> Result<()> {
        requeue(&self.client, age_minutes).await
    }

    fn describe(&self) -> String {
        "redis queue".to_string()
    }

    fn shutdown(&self) -> Result<()> {
        self.token.cancel();
        Ok(())
    }
}

/// Builds the [`RedisQueue`] provider (client, registry, run options, token)
/// from config. Factored out of [`create_provider`] so tests can inspect the
/// resulting `run_opts` without needing to downcast the opaque [`Queue`].
#[allow(clippy::unused_async)]
async fn build_provider(qcfg: &RedisQueueConfig) -> Result<RedisQueue> {
    let client = connect(&qcfg.uri)?;
    let registry = JobRegistry::new();
    let token = CancellationToken::new();
    let run_opts = RunOpts {
        num_workers: qcfg.num_workers,
        poll_interval_sec: 1,
        queues: qcfg.queues.clone(),
        reaper: qcfg.reaper.clone(),
    };
    debug!(
        queues = ?qcfg.queues,
        num_workers = qcfg.num_workers,
        "creating Redis queue provider"
    );
    Ok(RedisQueue {
        client,
        registry: Arc::new(tokio::sync::Mutex::new(registry)),
        run_opts,
        token,
    })
}

/// Create this provider
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn create_provider(qcfg: &RedisQueueConfig) -> Result<Queue> {
    Ok(Queue::from_provider(Arc::new(build_provider(qcfg).await?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bgworker::BackgroundWorker, tests_cfg::redis::setup_redis_container};
    use chrono::Utc;
    use testcontainers::{ContainerAsync, GenericImage};

    async fn setup_redis() -> (RedisPool, ContainerAsync<GenericImage>) {
        let (redis_url, container) = setup_redis_container().await;
        let client = connect(&redis_url).expect("connect to redis");
        (client, container)
    }

    async fn get_test_connection(client: &RedisPool) -> Connection {
        client
            .get_multiplexed_async_connection()
            .await
            .expect("get connection")
    }

    async fn redis_seed_data(client: &RedisPool) -> Result<()> {
        // Creating processed jobs
        let now = Utc::now();
        for i in 0..5 {
            let complete_job = Job {
                id: format!("job{i}"),
                name: "TestJob".to_string(),
                data: serde_json::json!({"counter": i}),
                status: JobStatus::Completed,
                run_at: now,
                interval: None,
                created_at: Some(now - chrono::Duration::days(15)),
                updated_at: Some(now - chrono::Duration::days(15)),
                tags: None,
                priority: 0,
            };

            let mut conn = get_connection(client).await?;
            // Store job data
            let _: () = conn
                .set(format!("{JOB_KEY_PREFIX}job{i}"), complete_job.to_json()?)
                .await?;
        }

        // Create queued jobs
        let args = serde_json::json!({"hello": "world"});
        enqueue(client, "TestJob".to_string(), None, args, None, None).await?;

        // Create job with tags
        let args = serde_json::json!({"hello": "tagged"});
        enqueue(
            client,
            "TaggedJob".to_string(),
            None,
            args,
            Some(vec!["important".to_string(), "urgent".to_string()]),
            None,
        )
        .await?;

        Ok(())
    }

    async fn get_all_jobs(client: &RedisPool) -> Vec<Job> {
        get_jobs(client, None, None).await.unwrap_or_default()
    }

    #[tokio::test]
    async fn test_can_dequeue_redis() {
        let (client, _container) = setup_redis().await;
        redis_seed_data(&client).await.expect("seed data");

        // Dequeue job
        let queues = vec!["default".to_string()];
        let mut conn = get_test_connection(&client).await;
        let job_opt = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue");

        // Verify job was dequeued
        assert!(job_opt.is_some());
    }

    #[tokio::test]
    async fn test_can_clear_redis() {
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Seed data
        if let Err(e) = redis_seed_data(&client).await {
            panic!("Failed to seed data: {e}");
        }

        // Verify data exists first
        let mut conn = get_connection(&client).await.expect("get connection");
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg("*")
            .query_async(&mut conn)
            .await
            .expect("get keys");
        assert!(!keys.is_empty(), "Should have keys before clearing");

        // Clear data
        assert!(clear(&client).await.is_ok());

        // Verify data is gone
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg("*")
            .query_async(&mut conn)
            .await
            .expect("get keys");
        assert!(keys.is_empty(), "All keys should be removed after clearing");
    }

    #[tokio::test]
    async fn test_can_enqueue_redis() {
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Test enqueue
        let args = serde_json::json!({"user_id": 42});
        assert!(
            enqueue(&client, "PasswordReset".to_string(), None, args, None, None)
                .await
                .is_ok()
        );

        // Verify job was created
        let jobs = get_all_jobs(&client).await;
        assert_eq!(jobs.len(), 1);

        let job = &jobs[0];
        assert_eq!(job.name, "PasswordReset");
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.data, serde_json::json!({"user_id": 42}));
    }

    #[tokio::test]
    async fn test_can_enqueue_with_queue_redis() {
        let (client, _container) = setup_redis().await;

        // Test enqueue with custom queue
        let args = serde_json::json!({"email": "user@example.com"});
        assert!(enqueue(
            &client,
            "EmailNotification".to_string(),
            Some("mailer".to_string()),
            args,
            None,
            None
        )
        .await
        .is_ok());

        // Verify job was created in correct queue first
        let mut conn = get_test_connection(&client).await;
        let queue_key = format!("{QUEUE_KEY_PREFIX}mailer");
        let queue_len: i64 = conn.zcard(&queue_key).await.expect("get queue length");
        assert_eq!(queue_len, 1);

        // Test dequeue from mailer queue
        let queues = vec!["mailer".to_string()];
        let _job_opt = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue");

        // Queue should now be empty
        let queue_len: i64 = conn.zcard(&queue_key).await.expect("get queue length");
        assert_eq!(queue_len, 0);
    }

    #[tokio::test]
    async fn test_can_enqueue_batch_redis() {
        let (client, _container) = setup_redis().await;
        assert!(clear(&client).await.is_ok());

        // Mixed per-job priorities: the default (None → 0), a high and a low
        // value. Dequeue order must follow priority, not insertion order.
        let jobs = vec![
            (serde_json::json!({"user_id": 1}), None),
            (serde_json::json!({"user_id": 2}), Some(10)),
            (serde_json::json!({"user_id": 3}), Some(-5)),
        ];
        let ids = enqueue_batch(&client, "BatchJob".to_string(), None, jobs, None)
            .await
            .expect("batch enqueue");
        assert_eq!(ids.len(), 3);

        // Every job key was written, in input order, with its own priority.
        let stored = get_all_jobs(&client).await;
        assert_eq!(stored.len(), 3);
        for (id, expected_priority) in ids.iter().zip([0, 10, -5]) {
            let job = stored
                .iter()
                .find(|job| &job.id == id)
                .expect("batched job must be stored");
            assert_eq!(job.name, "BatchJob");
            assert_eq!(job.status, JobStatus::Queued);
            assert_eq!(job.priority, expected_priority);
        }

        // Every id landed in the default queue's ZSET.
        let mut conn = get_test_connection(&client).await;
        let queue_key = format!("{QUEUE_KEY_PREFIX}default");
        let queue_len: i64 = conn.zcard(&queue_key).await.expect("get queue length");
        assert_eq!(queue_len, 3);

        let queues = vec!["default".to_string()];
        for expected_user in [2, 1, 3] {
            let (job, _) = dequeue_with_conn(&mut conn, &queues, &[])
                .await
                .expect("dequeue")
                .expect("a batched job must be dequeueable");
            assert_eq!(
                job.data.get("user_id"),
                Some(&serde_json::json!(expected_user)),
                "batched jobs must be dequeued by priority"
            );
            complete_job_with_conn(&mut conn, &job.id, "default", None)
                .await
                .expect("complete job");
        }
        assert!(dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue")
            .is_none());
    }

    #[tokio::test]
    async fn test_can_enqueue_batch_with_queue_and_tags_redis() {
        let (client, _container) = setup_redis().await;
        assert!(clear(&client).await.is_ok());

        let ids = enqueue_batch(
            &client,
            "BatchJob".to_string(),
            Some("mailer".to_string()),
            vec![
                (serde_json::json!({"user_id": 1}), None),
                (serde_json::json!({"user_id": 2}), None),
            ],
            Some(vec!["email".to_string()]),
        )
        .await
        .expect("tagged batch enqueue");
        assert_eq!(ids.len(), 2);

        // The batch went to the named queue, not the default one.
        let mut conn = get_test_connection(&client).await;
        let mailer_len: i64 = conn
            .zcard(format!("{QUEUE_KEY_PREFIX}mailer"))
            .await
            .expect("get queue length");
        assert_eq!(mailer_len, 2);
        let default_len: i64 = conn
            .zcard(format!("{QUEUE_KEY_PREFIX}default"))
            .await
            .expect("get queue length");
        assert_eq!(default_len, 0);

        // Tags apply to every job: a tagless worker must not see them and a
        // worker carrying the tag must.
        let queues = vec!["mailer".to_string()];
        assert!(
            dequeue_with_conn(&mut conn, &queues, &[])
                .await
                .expect("dequeue")
                .is_none(),
            "an untagged worker must not see tagged jobs"
        );
        for _ in 0..2 {
            let (job, _) = dequeue_with_conn(&mut conn, &queues, &["email".to_string()])
                .await
                .expect("dequeue")
                .expect("a tagged batched job must be dequeueable by a matching worker");
            assert!(ids.contains(&job.id));
            assert_eq!(job.tags, Some(vec!["email".to_string()]));
            complete_job_with_conn(&mut conn, &job.id, "mailer", None)
                .await
                .expect("complete job");
        }
    }

    #[tokio::test]
    async fn test_can_complete_job_redis() {
        let (client, _container) = setup_redis().await;

        // Add job
        let args = serde_json::json!({"task": "test"});
        assert!(
            enqueue(&client, "TestJob".to_string(), None, args, None, None)
                .await
                .is_ok()
        );

        // Dequeue job
        let queues = vec!["default".to_string()];
        let mut conn = get_test_connection(&client).await;
        let job_opt = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue");
        let (job, queue) = job_opt.unwrap();

        // Complete job
        assert!(complete_job_with_conn(&mut conn, &job.id, &queue, None)
            .await
            .is_ok());

        // Verify job is not in processing set
        let processing_key = format!("{PROCESSING_KEY_PREFIX}{queue}");
        let is_member: bool = conn
            .sismember(&processing_key, &job.id)
            .await
            .expect("check membership");
        assert!(!is_member);

        // Verify job status is updated to Completed
        let job_key = String::from(JOB_KEY_PREFIX) + &job.id;
        let job_json: String = conn.get(&job_key).await.expect("get job");
        let completed_job = Job::from_json(&job_json).expect("parse job");
        assert_eq!(
            completed_job.status,
            JobStatus::Completed,
            "Job status should be Completed after completion"
        );
    }

    #[tokio::test]
    async fn test_can_complete_job_with_interval_redis() {
        let (client, _container) = setup_redis().await;

        // Add job
        let args = serde_json::json!({"task": "recurring"});
        assert!(
            enqueue(&client, "RecurringJob".to_string(), None, args, None, None)
                .await
                .is_ok()
        );

        // Dequeue job
        let queues = vec!["default".to_string()];
        let mut conn = get_test_connection(&client).await;
        let job_opt = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue");
        let (job, queue) = job_opt.unwrap();

        // Complete job with interval to reschedule
        assert!(
            complete_job_with_conn(&mut conn, &job.id, &queue, Some(1000))
                .await
                .is_ok()
        );

        // Verify job is back in queue
        let queue_key = format!("{QUEUE_KEY_PREFIX}{queue}");
        let queue_len: i64 = conn.zcard(&queue_key).await.expect("get queue length");
        assert_eq!(queue_len, 1);

        // Get the job ID from the queue (ZSET - first element by score)
        let job_ids: Vec<String> = conn.zrange(&queue_key, 0, 0).await.expect("get job id");
        let job_id = job_ids.first().expect("job should exist").clone();

        // Get the job data using the ID
        let job_key = format!("{JOB_KEY_PREFIX}{job_id}");
        let job_json: String = conn.get(&job_key).await.expect("get job data");
        let requeued_job = Job::from_json(&job_json).expect("parse job");

        // Verify the job has future run_at time
        assert!(requeued_job.run_at > Utc::now());
    }

    #[tokio::test]
    async fn test_can_fail_job_redis() {
        let (client, _container) = setup_redis().await;

        // Add job
        let args = serde_json::json!({"task": "test"});
        assert!(
            enqueue(&client, "TestJob".to_string(), None, args, None, None)
                .await
                .is_ok()
        );

        // Dequeue job
        let queues = vec!["default".to_string()];
        let mut conn = get_test_connection(&client).await;
        let job_opt = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue");
        let (job, queue) = job_opt.unwrap();

        // Fail job
        let error = Error::string("test failure");
        assert!(fail_job_with_conn(&mut conn, &job.id, &queue, &error)
            .await
            .is_ok());

        // Verify job is not in processing set
        let processing_key = format!("{PROCESSING_KEY_PREFIX}{queue}");
        let is_member: bool = conn
            .sismember(&processing_key, &job.id)
            .await
            .expect("check membership");
        assert!(!is_member);

        // Verify job has error data
        let job_key = String::from(JOB_KEY_PREFIX) + &job.id;
        let job_json: String = conn.get(&job_key).await.expect("get job");
        let failed_job = Job::from_json(&job_json).expect("parse job");
        assert_eq!(failed_job.status, JobStatus::Failed);
        assert!(failed_job.data.get("error").is_some());
    }

    /// A failed job must stay visible to `get_jobs`.
    ///
    /// `fail_job` removes the id from the processing set and adds it to no
    /// queue, so a driver that enumerates jobs by walking queue/processing keys
    /// loses the job the moment it fails — taking `jobs dump --status failed`,
    /// `jobs purge` and `clear_by_status` with it.
    ///
    /// `test_can_get_jobs_redis` cannot catch this: it asserts inside
    /// `for job in &failed_jobs`, which passes vacuously on an empty list.
    #[tokio::test]
    async fn a_failed_job_is_still_listed() {
        let (client, _container) = setup_redis().await;

        enqueue(
            &client,
            "TestJob".to_string(),
            None,
            serde_json::json!({"hello": "world"}),
            None,
            None,
        )
        .await
        .expect("enqueue");

        let mut conn = get_test_connection(&client).await;
        let (job, queue) = dequeue_with_conn(&mut conn, &["default".to_string()], &[])
            .await
            .expect("dequeue")
            .expect("a job to dequeue");
        fail_job_with_conn(&mut conn, &job.id, &queue, &Error::string("test failure"))
            .await
            .expect("fail the job");

        let failed = get_jobs(&client, Some(&vec![JobStatus::Failed]), None)
            .await
            .expect("get failed jobs");

        assert_eq!(failed.len(), 1, "the failed job must still be listed");
        assert_eq!(failed[0].id, job.id);
    }

    #[tokio::test]
    async fn can_retry_a_failed_job() {
        let (client, _container) = setup_redis().await;

        enqueue(
            &client,
            "TestJob".to_string(),
            None,
            serde_json::json!({"hello": "world"}),
            None,
            None,
        )
        .await
        .expect("enqueue");

        let mut conn = get_test_connection(&client).await;
        let (job, queue) = dequeue_with_conn(&mut conn, &["default".to_string()], &[])
            .await
            .expect("dequeue")
            .expect("a job to dequeue");
        fail_job_with_conn(&mut conn, &job.id, &queue, &Error::string("test failure"))
            .await
            .expect("fail the job");

        assert_eq!(retry_failed(&client, None).await.expect("retry"), 1);
        assert!(get_jobs(&client, Some(&vec![JobStatus::Failed]), None)
            .await
            .expect("get failed")
            .is_empty());

        // Queued is not enough: the id must be back in a queue ZSET, or no
        // worker will ever see it again.
        let (again, _) = dequeue_with_conn(&mut conn, &["default".to_string()], &[])
            .await
            .expect("dequeue")
            .expect("the retried job is dequeueable");
        assert_eq!(again.id, job.id);
        // The failure trail is kept — a retry should not erase why it failed.
        assert!(again.data.get("error").is_some());

        assert_eq!(
            retry_failed(&client, Some("nonexistent"))
                .await
                .expect("retry"),
            0
        );
    }

    #[tokio::test]
    async fn test_can_get_jobs_redis() {
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Seed data
        redis_seed_data(&client).await.expect("seed data");

        // The seed writes 5 completed jobs and enqueues 2 queued ones. Every
        // count below is asserted, not just the per-item status: the previous
        // version only checked `for job in &list { assert_eq!(job.status, ..) }`,
        // which passes on an empty list — and the completed and failed lists
        // *were* empty, because `get_jobs` walked the queues rather than the
        // job keys and so could not see a job that had stopped being runnable.
        let all_jobs = get_jobs(&client, None, None).await.expect("get all jobs");
        assert_eq!(all_jobs.len(), 7, "5 completed + 2 queued");

        let queued_jobs = get_jobs(&client, Some(&vec![JobStatus::Queued]), None)
            .await
            .expect("get queued jobs");
        assert_eq!(queued_jobs.len(), 2);
        for job in &queued_jobs {
            assert_eq!(job.status, JobStatus::Queued);
        }

        let completed_jobs = get_jobs(&client, Some(&vec![JobStatus::Completed]), None)
            .await
            .expect("get completed jobs");
        assert_eq!(completed_jobs.len(), 5);
        for job in &completed_jobs {
            assert_eq!(job.status, JobStatus::Completed);
        }

        let failed_jobs = get_jobs(&client, Some(&vec![JobStatus::Failed]), None)
            .await
            .expect("get failed jobs");
        assert!(failed_jobs.is_empty(), "the seed fails nothing");

        // Verify combined status filter
        let combined_jobs = get_jobs(
            &client,
            Some(&vec![JobStatus::Completed, JobStatus::Failed]),
            None,
        )
        .await
        .expect("get combined jobs");
        assert_eq!(combined_jobs.len(), 5);
        for job in &combined_jobs {
            assert!(job.status == JobStatus::Completed || job.status == JobStatus::Failed);
        }
    }

    #[tokio::test]
    async fn test_job_registry_redis() {
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Create job registry
        let mut registry = JobRegistry::new();

        // Create a mock worker
        struct TestWorker;
        #[async_trait::async_trait]
        impl BackgroundWorker<String> for TestWorker {
            fn build(_ctx: &crate::app::AppContext) -> Self {
                Self
            }

            async fn perform(&self, args: String) -> crate::Result<()> {
                assert_eq!(args, "test args");
                Ok(())
            }
        }

        // Register worker
        let handler = crate::bgworker::erase_worker(TestWorker);
        assert!(registry
            .insert_handler("TestJob".to_string(), handler)
            .is_ok());

        // Add job
        let args = serde_json::json!("test args");
        assert!(
            enqueue(&client, "TestJob".to_string(), None, args, None, None)
                .await
                .is_ok()
        );

        // Run registry with worker for a short time
        let opts = RunOpts {
            num_workers: 1,
            poll_interval_sec: 1,
            queues: None,
            reaper: None,
        };

        let token = CancellationToken::new();
        let worker_handles = registry.run(&client, &opts, &token, &[] as &[String]);

        // Allow some time for job processing
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Stop workers
        token.cancel();
        for handle in worker_handles {
            let _ = handle.await;
        }
    }

    #[tokio::test]
    async fn test_job_filtering_by_tags() {
        let (client, _container) = setup_redis().await;

        // Clear any existing data for clean test environment
        assert!(clear(&client).await.is_ok());

        // Create jobs with different tags using the proper enqueue function
        let args1 = serde_json::json!({"task": "task1"});
        assert!(enqueue(
            &client,
            "TaggedJob".to_string(),
            Some("default".to_string()),
            args1,
            Some(vec!["tag1".to_string(), "common".to_string()]),
            None
        )
        .await
        .is_ok());

        let args2 = serde_json::json!({"task": "task2"});
        assert!(enqueue(
            &client,
            "TaggedJob".to_string(),
            Some("default".to_string()),
            args2,
            Some(vec!["tag2".to_string(), "common".to_string()]),
            None
        )
        .await
        .is_ok());

        let args3 = serde_json::json!({"task": "task3"});
        assert!(enqueue(
            &client,
            "TaggedJob".to_string(),
            Some("default".to_string()),
            args3,
            Some(vec!["tag3".to_string()]),
            None
        )
        .await
        .is_ok());

        // Test dequeue with tag1 filter
        let queues = vec!["default".to_string()];
        let mut conn = get_test_connection(&client).await;
        let job_opt = dequeue_with_conn(&mut conn, &queues, &["tag1".to_string()])
            .await
            .expect("dequeue with tag1");

        assert!(job_opt.is_some(), "Should have found a job with tag1");
        if let Some((dequeued_job, _)) = job_opt {
            assert_eq!(dequeued_job.name, "TaggedJob");
            assert!(dequeued_job.tags.is_some(), "Job should have tags");
            let tags = dequeued_job.tags.unwrap();
            assert!(
                tags.contains(&"tag1".to_string()),
                "Job should contain tag1"
            );
        }
    }

    #[tokio::test]
    async fn test_ping_redis() {
        let (client, _container) = setup_redis().await;
        ping(&client).await.expect("ping redis");
    }

    #[tokio::test]
    async fn test_can_clear_by_status_redis() {
        // Setup Redis directly with testcontainer using the reliable method
        let (client, _container) = setup_redis().await;

        // Seed data with error handling
        match redis_seed_data(&client).await {
            Ok(()) => (),
            Err(e) => panic!("Failed to seed data: {e}"),
        }

        // Count jobs by status before clearing
        let all_jobs = get_all_jobs(&client).await;
        let completed_count = all_jobs
            .iter()
            .filter(|j| j.status == JobStatus::Completed)
            .count();
        let failed_count = all_jobs
            .iter()
            .filter(|j| j.status == JobStatus::Failed)
            .count();
        let total_count = all_jobs.len();

        // Clear completed and failed jobs
        assert!(
            clear_by_status(&client, vec![JobStatus::Completed, JobStatus::Failed])
                .await
                .is_ok()
        );

        // Verify jobs were cleared
        let remaining_jobs = get_all_jobs(&client).await;
        assert_eq!(
            remaining_jobs.len(),
            total_count - completed_count - failed_count
        );
        assert_eq!(
            remaining_jobs
                .iter()
                .filter(|j| j.status == JobStatus::Completed)
                .count(),
            0
        );
        assert_eq!(
            remaining_jobs
                .iter()
                .filter(|j| j.status == JobStatus::Failed)
                .count(),
            0
        );
    }

    #[tokio::test]
    async fn test_can_clear_jobs_older_than_with_status_redis() {
        // Setup with clean Redis
        let (client, _container) = setup_redis().await;

        // Add specific test jobs with known ages and statuses
        let mut conn = get_connection(&client).await.expect("get connection");

        // Create an old failed job (older than 10 days)
        let old_failed_job = Job {
            id: "old_failed_job_test".to_string(),
            name: "OldFailedTestJob".to_string(),
            data: serde_json::json!({"test": "data"}),
            status: JobStatus::Failed,
            run_at: Utc::now(),
            interval: None,
            created_at: Some(Utc::now() - chrono::Duration::days(15)),
            updated_at: Some(Utc::now() - chrono::Duration::days(15)),
            tags: None,
            priority: 0,
        };

        // Create an old completed job (older than 10 days)
        let old_completed_job = Job {
            id: "old_completed_job_test".to_string(),
            name: "OldCompletedTestJob".to_string(),
            data: serde_json::json!({"test": "data"}),
            status: JobStatus::Completed,
            run_at: Utc::now(),
            interval: None,
            created_at: Some(Utc::now() - chrono::Duration::days(15)),
            updated_at: Some(Utc::now() - chrono::Duration::days(15)),
            tags: None,
            priority: 0,
        };

        // Store both jobs directly
        let old_failed_job_json = old_failed_job.to_json().expect("serialize old failed job");
        let old_completed_job_json = old_completed_job
            .to_json()
            .expect("serialize old completed job");

        let old_failed_job_key = String::from(JOB_KEY_PREFIX) + &old_failed_job.id;
        let old_completed_job_key = String::from(JOB_KEY_PREFIX) + &old_completed_job.id;

        let _: () = conn
            .set(&old_failed_job_key, &old_failed_job_json)
            .await
            .expect("set old failed job");
        let _: () = conn
            .set(&old_completed_job_key, &old_completed_job_json)
            .await
            .expect("set old completed job");

        // Clear only failed jobs older than 10 days
        assert!(
            clear_jobs_older_than(&client, 10, Some(&vec![JobStatus::Failed]))
                .await
                .is_ok()
        );

        // Check if old failed job was removed and old completed job still exists
        let exists_failed_after: bool = conn
            .exists(&old_failed_job_key)
            .await
            .expect("check failed job after");
        let exists_completed_after: bool = conn
            .exists(&old_completed_job_key)
            .await
            .expect("check completed job after");

        assert!(!exists_failed_after, "Old failed job should be removed");
        assert!(
            exists_completed_after,
            "Old completed job should still exist"
        );
    }

    #[tokio::test]
    async fn test_can_get_jobs_with_age_redis() {
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Seed data with jobs of different ages
        redis_seed_data(&client).await.expect("seed data");

        // Get jobs older than 10 days
        let old_jobs = get_jobs(&client, None, Some(10))
            .await
            .expect("get old jobs");
        for job in &old_jobs {
            if let Some(created_at) = job.created_at {
                assert!(created_at <= Utc::now() - chrono::Duration::days(10));
            }
        }

        // Get old jobs with specific status
        let old_failed_jobs = get_jobs(&client, Some(&vec![JobStatus::Failed]), Some(10))
            .await
            .expect("get old failed jobs");
        for job in &old_failed_jobs {
            assert_eq!(job.status, JobStatus::Failed);
            if let Some(created_at) = job.created_at {
                assert!(created_at <= Utc::now() - chrono::Duration::days(10));
            }
        }
    }

    #[tokio::test]
    async fn test_priority_ordering_redis() {
        let (client, _container) = setup_redis().await;
        assert!(clear(&client).await.is_ok());

        // Base time in the past so all jobs are ready. Expected dequeue order by
        // `index`: 1) i32::MAX, 2) prio 42 (earlier), 3) prio 42 (later),
        // 4) prio 0, 5) i32::MIN.
        let base_time = Utc::now() - chrono::Duration::minutes(10);
        let mut conn = get_test_connection(&client).await;
        let queue_key = format!("{QUEUE_KEY_PREFIX}default");

        let seeds = [
            ("job1", "Task1", i32::MAX, 4_i64, 1),
            ("job2", "Task2", 42, 1, 2),
            ("job3", "Task3", 42, 3, 3),
            ("job4", "Task4", 0, 0, 4),
            ("job5", "Task5", i32::MIN, 2, 5),
        ];
        for (id, name, priority, minute_offset, index) in seeds {
            let mut job = Job::new(
                id.to_string(),
                name.to_string(),
                serde_json::json!({ "index": index }),
            );
            job.priority = priority;
            job.run_at = base_time + chrono::Duration::minutes(minute_offset);
            let score = calculate_score(job.priority);
            let _: () = conn
                .set(format!("{JOB_KEY_PREFIX}{id}"), job.to_json().unwrap())
                .await
                .unwrap();
            let _: () = conn.zadd(&queue_key, id, score).await.unwrap();
        }

        let queues = vec!["default".to_string()];
        for expected_index in [1, 2, 3, 4, 5] {
            let (job, _) = dequeue_with_conn(&mut conn, &queues, &[])
                .await
                .expect("dequeue failed")
                .expect("expected a job");
            assert_eq!(
                job.data.get("index"),
                Some(&serde_json::json!(expected_index))
            );
            complete_job_with_conn(&mut conn, &job.id, "default", None)
                .await
                .expect("complete job");
        }

        let job_opt = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue failed");
        assert!(job_opt.is_none());
    }

    #[tokio::test]
    async fn test_enqueue_with_priority_redis() {
        let (client, _container) = setup_redis().await;
        assert!(clear(&client).await.is_ok());

        let args = serde_json::json!({"user_id": 1});
        enqueue(
            &client,
            "PriorityJob".to_string(),
            None,
            args.clone(),
            None,
            Some(42),
        )
        .await
        .expect("enqueue with priority");
        enqueue(
            &client,
            "DefaultPriorityJob".to_string(),
            None,
            args,
            None,
            None,
        )
        .await
        .expect("enqueue without priority");

        let jobs = get_all_jobs(&client).await;
        assert_eq!(jobs.len(), 2);
        assert_eq!(
            jobs.iter()
                .find(|j| j.name == "PriorityJob")
                .expect("PriorityJob")
                .priority,
            42
        );
        assert_eq!(
            jobs.iter()
                .find(|j| j.name == "DefaultPriorityJob")
                .expect("DefaultPriorityJob")
                .priority,
            0
        );
    }

    #[tokio::test]
    async fn test_negative_priority_redis() {
        let (client, _container) = setup_redis().await;
        assert!(clear(&client).await.is_ok());

        enqueue(
            &client,
            "NegativePriorityJob".to_string(),
            None,
            serde_json::json!({"task": "negative_priority"}),
            None,
            Some(-10),
        )
        .await
        .expect("enqueue negative priority");
        enqueue(
            &client,
            "ZeroPriorityJob".to_string(),
            None,
            serde_json::json!({"task": "zero_priority"}),
            None,
            Some(0),
        )
        .await
        .expect("enqueue zero priority");

        let queues = vec!["default".to_string()];
        let mut conn = get_test_connection(&client).await;

        // Zero priority is dequeued before the negative one.
        let (job, _) = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue failed")
            .expect("expected a job");
        assert_eq!(job.priority, 0);
        assert_eq!(job.name, "ZeroPriorityJob");
        complete_job_with_conn(&mut conn, &job.id, "default", None)
            .await
            .expect("complete job");

        let (job, _) = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue failed")
            .expect("expected a job");
        assert_eq!(job.priority, -10);
        assert_eq!(job.name, "NegativePriorityJob");
    }

    #[tokio::test]
    async fn test_dequeue_skips_mismatched_tags_no_infinite_loop() {
        let (client, _container) = setup_redis().await;
        assert!(clear(&client).await.is_ok());

        let mut conn = get_test_connection(&client).await;
        let queue_key = format!("{QUEUE_KEY_PREFIX}default");

        // A tagged job at the front (older run_at) that a no-tag worker skips.
        let mut job1 = Job::new(
            "job1".to_string(),
            "TaggedJob".to_string(),
            serde_json::json!({"task": "tagged"}),
        );
        job1.tags = Some(vec!["tag1".to_string()]);
        job1.run_at = Utc::now() - chrono::Duration::hours(1);
        let score1 = calculate_score(job1.priority);
        let _: () = conn
            .set(format!("{JOB_KEY_PREFIX}job1"), job1.to_json().unwrap())
            .await
            .unwrap();
        let _: () = conn.zadd(&queue_key, "job1", score1).await.unwrap();

        // An untagged job behind it that should be picked up.
        let mut job2 = Job::new(
            "job2".to_string(),
            "UntaggedJob".to_string(),
            serde_json::json!({"task": "untagged"}),
        );
        job2.tags = None;
        job2.run_at = Utc::now() - chrono::Duration::minutes(30);
        let score2 = calculate_score(job2.priority);
        let _: () = conn
            .set(format!("{JOB_KEY_PREFIX}job2"), job2.to_json().unwrap())
            .await
            .unwrap();
        let _: () = conn.zadd(&queue_key, "job2", score2).await.unwrap();

        let queues = vec!["default".to_string()];
        let (job, _) = dequeue_with_conn(&mut conn, &queues, &[])
            .await
            .expect("dequeue")
            .expect("should have dequeued the untagged job");
        assert_eq!(job.id, "job2", "Should have picked job2");
    }

    // `Client::open` does not eagerly connect, so these wiring tests don't
    // need a running Redis instance.
    #[tokio::test]
    async fn create_provider_wires_reaper_config() {
        let qcfg = RedisQueueConfig {
            uri: "redis://127.0.0.1:6379".to_string(),
            dangerously_flush: false,
            queues: None,
            num_workers: 1,
            reaper: Some(ReaperConfig {
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
        let qcfg = RedisQueueConfig {
            uri: "redis://127.0.0.1:6379".to_string(),
            dangerously_flush: false,
            queues: None,
            num_workers: 1,
            reaper: None,
        };

        let provider = build_provider(&qcfg).await.expect("build provider");
        assert!(provider.run_opts.reaper.is_none());
    }
}
