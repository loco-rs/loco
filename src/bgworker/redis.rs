/// Redis based background job queue provider
use std::{
    collections::HashMap, future::Future, panic::AssertUnwindSafe, pin::Pin, sync::Arc,
    time::Duration,
};

use super::{BackgroundWorker, JobStatus, Queue};
use crate::{config::RedisQueueConfig, Error, Result};
use chrono::{DateTime, Utc};
use futures_util::FutureExt;
use redis::{aio::MultiplexedConnection as Connection, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};
use ulid::Ulid;

pub type RedisPool = Client;
type JobId = String;
type JobData = JsonValue;

const QUEUE_KEY_PREFIX: &str = "queue:";
const JOB_KEY_PREFIX: &str = "job:";
const PROCESSING_KEY_PREFIX: &str = "processing:";

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
    pub tags: Option<Vec<String>>,
}

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

    /// Registers a job handler with the provided name.
    ///
    /// # Errors
    ///
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
                    Ok(args) => {
                        // Wrap the perform call in catch_unwind to handle panics
                        match AssertUnwindSafe(w.perform(args)).catch_unwind().await {
                            Ok(result) => result,
                            Err(panic) => {
                                let panic_msg = panic
                                    .downcast_ref::<String>()
                                    .map(String::as_str)
                                    .or_else(|| panic.downcast_ref::<&str>().copied())
                                    .unwrap_or("Unknown panic occurred");
                                error!(err = panic_msg, "worker panicked");
                                Err(Error::string(panic_msg))
                            }
                        }
                    }
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
                loop {
                    // Check for cancellation before potentially blocking on dequeue
                    if worker_token.is_cancelled() {
                        trace!(worker_num = idx, "cancellation received, stopping worker");
                        break;
                    }

                    let job_opt = match dequeue(&client, &queues, &tags).await {
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
                                    if let Err(err) =
                                        complete_job(&client, &job.id, &queue_name, job.interval)
                                            .await
                                    {
                                        error!(
                                            err = err.to_string(),
                                            job = ?job,
                                            "cannot complete job"
                                        );
                                    }
                                }
                                Err(err) => {
                                    if let Err(err) =
                                        fail_job(&client, &job.id, &queue_name, &err).await
                                    {
                                        error!(
                                            err = err.to_string(),
                                            job = ?job,
                                            "cannot fail job"
                                        );
                                    }
                                }
                            }
                        } else {
                            error!(job = job.name, "no handler found for job");
                        }
                    } else {
                        // Use tokio::select! to wait for interval or cancellation
                        tokio::select! {
                            biased;
                            () = worker_token.cancelled() => {
                                trace!(worker_num = idx, "cancellation received during sleep, stopping worker");
                                break;
                            }
                            () = sleep(Duration::from_secs(interval.into())) => {
                                // Interval elapsed, continue loop
                            }
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
) -> Result<()> {
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

    // Serialize job for Redis storage
    let job_json = job.to_json()?;

    // Store job in Redis queue and in job key
    let job_key = String::from(JOB_KEY_PREFIX) + &job.id;
    redis::pipe()
        .rpush(queue_key, &job_json)
        .set(job_key, &job_json)
        .query_async::<()>(&mut conn)
        .await?;

    Ok(())
}

async fn dequeue(
    client: &RedisPool,
    queues: &[String],
    tags: &[String],
) -> Result<Option<(Job, String)>> {
    if queues.is_empty() {
        return Ok(None);
    }

    let mut conn = get_connection(client).await?;

    // Try to get a job from each queue in order (round-robin is more complex)
    for queue_name in queues {
        let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");

        // Use LPOP to get and remove the first job from the queue
        let job_json: Option<String> = conn.lpop(&queue_key, None).await?;

        if let Some(json) = job_json {
            match Job::from_json(&json) {
                Ok(job) => {
                    // Check tag filtering
                    let should_process = if tags.is_empty() {
                        // Worker has no tags - only process jobs with no tags
                        job.tags.is_none() || job.tags.as_ref().is_some_and(Vec::is_empty)
                    } else {
                        // Worker has tags - only process jobs with matching tags
                        job.tags
                            .as_ref()
                            .is_some_and(|job_tags| job_tags.iter().any(|tag| tags.contains(tag)))
                    };

                    if !should_process {
                        // Put the job back in the queue
                        let _: () = conn.rpush(&queue_key, json).await?;
                        trace!(
                            job_id = job.id,
                            job_tags = ?job.tags,
                            worker_tags = ?tags,
                            "Job doesn't match tag criteria, returning to queue"
                        );
                        continue;
                    }

                    // Store job ID in processing set
                    let processing_key = format!("{PROCESSING_KEY_PREFIX}{queue_name}");
                    let _: () = conn.sadd(&processing_key, &job.id).await?;

                    return Ok(Some((job, queue_name.clone())));
                }
                Err(err) => {
                    error!(err = err.to_string(), "failed to parse job JSON");
                }
            }
        }
    }

    Ok(None)
}

async fn complete_job(
    client: &RedisPool,
    id: &JobId,
    queue_name: &str,
    interval_ms: Option<i64>,
) -> Result<()> {
    let mut conn = get_connection(client).await?;
    let processing_key = format!("{PROCESSING_KEY_PREFIX}{queue_name}");

    // Remove job from processing set
    let _: () = redis::pipe()
        .srem(&processing_key, id)
        .query_async(&mut conn)
        .await?;

    // Get job details
    let job_key = String::from(JOB_KEY_PREFIX) + id;
    let job_json: Option<String> = conn.get(&job_key).await?;

    if let Some(json) = job_json {
        if let Ok(mut job) = Job::from_json(&json) {
            // If the job has an interval, requeue it
            if let Some(interval) = interval_ms {
                // Update run_at time for the job
                job.run_at = Utc::now() + chrono::Duration::milliseconds(interval);

                // Reserialize and push to queue
                let new_json = job.to_json()?;
                let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");

                let _: () = redis::pipe()
                    .rpush(queue_key, new_json.clone())
                    .set(&job_key, new_json)
                    .query_async(&mut conn)
                    .await?;
            } else {
                // No interval, mark as completed
                job.status = JobStatus::Completed;
                job.updated_at = Some(Utc::now());

                // Save updated job
                let updated_json = job.to_json()?;
                let _: () = conn.set(&job_key, updated_json).await?;
            }
        }
    }

    Ok(())
}

async fn fail_job(
    client: &RedisPool,
    id: &JobId,
    queue_name: &str,
    error: &crate::Error,
) -> Result<()> {
    let mut conn = get_connection(client).await?;
    let processing_key = format!("{PROCESSING_KEY_PREFIX}{queue_name}");

    // Remove job from processing set
    let _: () = redis::pipe()
        .srem(&processing_key, id)
        .query_async(&mut conn)
        .await?;

    // Store the error with the job
    let job_key = String::from(JOB_KEY_PREFIX) + id;
    let job_json: Option<String> = conn.get(&job_key).await?;

    if let Some(json) = job_json {
        if let Ok(mut job) = Job::from_json(&json) {
            // Add error to job data
            let error_json = serde_json::json!({ "error": error.to_string() });
            job.data = error_json;
            job.status = JobStatus::Failed;

            // Save updated job
            let updated_json = job.to_json()?;
            let _: () = conn.set(&job_key, updated_json).await?;
        }
    }

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

/// Retrieves a list of jobs from the Redis queues.
///
/// This function queries Redis for jobs, optionally filtering by their
/// `status` and age. It will search through all processing sets and queue keys
/// to find jobs matching the criteria.
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
    let mut jobs = Vec::new();

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

    // Collect jobs from queues
    for queue_key in queue_keys {
        let queue_jobs: Vec<String> = conn.lrange(&queue_key, 0, -1).await?;
        for job_json in queue_jobs {
            if let Ok(job) = Job::from_json(&job_json) {
                if should_include_job(&job, status, age_days) {
                    jobs.push(job);
                }
            }
        }
    }

    // Collect jobs from processing sets
    for processing_key in processing_keys {
        let job_ids: Vec<String> = conn.smembers(&processing_key).await?;
        for job_id in job_ids {
            // Get the job from the job_key using the ID
            let job_key = String::from(JOB_KEY_PREFIX) + &job_id;
            let job_json: Option<String> = conn.get(&job_key).await?;

            if let Some(json) = job_json {
                if let Ok(mut job) = Job::from_json(&json) {
                    // Jobs in processing sets have status "queued" but should be "processing"
                    if job.status == JobStatus::Queued {
                        job.status = JobStatus::Processing;
                    }

                    if should_include_job(&job, status, age_days) {
                        jobs.push(job);
                    }
                }
            }
        }
    }

    Ok(jobs)
}

// Helper function to check if a job matches the filter criteria
fn should_include_job(job: &Job, status: Option<&Vec<JobStatus>>, age_days: Option<i64>) -> bool {
    // Check status filter
    if let Some(status_list) = status {
        if !status_list.contains(&job.status) {
            return false;
        }
    }

    // Check age filter
    if let Some(age_days) = age_days {
        if let Some(created_at) = job.created_at {
            let cutoff_date = Utc::now() - chrono::Duration::days(age_days);
            if created_at > cutoff_date {
                return false;
            }
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
        let queue_jobs: Vec<String> = conn.lrange(&queue_key, 0, -1).await?;

        // Process each job individually
        for job_json in queue_jobs {
            if let Ok(job) = Job::from_json(&job_json) {
                if status.contains(&job.status) {
                    // Remove this specific job from the queue
                    let _: i32 = conn.lrem(&queue_key, 1, &job_json).await?;

                    // Delete the job_key
                    let job_key = String::from(JOB_KEY_PREFIX) + &job.id;
                    let _: () = conn.del(&job_key).await?;
                }
            }
        }
    }

    // Process jobs in processing sets
    for processing_key in processing_keys {
        let job_ids: Vec<String> = conn.smembers(&processing_key).await?;

        for job_id in job_ids {
            // Get the job from the job_key using the ID
            let job_key = String::from(JOB_KEY_PREFIX) + &job_id;
            let job_json: Option<String> = conn.get(&job_key).await?;

            if let Some(json) = job_json {
                if let Ok(mut job) = Job::from_json(&json) {
                    // Jobs in processing sets have status "queued" but should be "processing"
                    if job.status == JobStatus::Queued {
                        job.status = JobStatus::Processing;
                    }

                    if status.contains(&job.status) {
                        // Remove job from the processing set
                        let _: i32 = conn.srem(&processing_key, &job_id).await?;

                        // Delete the job key
                        let _: () = conn.del(&job_key).await?;
                    }
                }
            }
        }
    }

    // Process standalone job keys that might not be in any queue or processing set
    // (e.g., completed, failed, or cancelled jobs)
    for job_key in job_keys {
        let job_json: Option<String> = conn.get(&job_key).await?;

        if let Some(json) = job_json {
            if let Ok(job) = Job::from_json(&json) {
                if status.contains(&job.status) {
                    // Delete the job key
                    let _: () = conn.del(&job_key).await?;
                }
            }
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
        let queue_jobs: Vec<String> = conn.lrange(&queue_key, 0, -1).await?;

        // Process each job individually
        for job_json in queue_jobs {
            if let Ok(job) = Job::from_json(&job_json) {
                // Check if the job should be removed based on age and status
                let should_remove = job.created_at.is_some_and(|created_at| {
                    created_at < cutoff_date && status.map_or(true, |s| s.contains(&job.status))
                });

                if should_remove {
                    // Remove this specific job from the queue
                    let _: i32 = conn.lrem(&queue_key, 1, &job_json).await?;

                    // Delete the job key
                    let job_key = String::from(JOB_KEY_PREFIX) + &job.id;
                    let _: () = conn.del(&job_key).await?;
                }
            }
        }
    }

    // Process jobs in processing sets
    for processing_key in processing_keys {
        let job_ids: Vec<String> = conn.smembers(&processing_key).await?;

        for job_id in job_ids {
            // Get the actual job data using the ID
            let job_key = String::from(JOB_KEY_PREFIX) + &job_id;
            let job_json: Option<String> = conn.get(&job_key).await?;

            if let Some(json) = job_json {
                if let Ok(mut job) = Job::from_json(&json) {
                    // Jobs in processing sets have status "queued" but should be "processing"
                    if job.status == JobStatus::Queued {
                        job.status = JobStatus::Processing;
                    }

                    // Check if the job should be removed based on age and status
                    let should_remove = job.created_at.is_some_and(|created_at| {
                        created_at < cutoff_date && status.map_or(true, |s| s.contains(&job.status))
                    });

                    if should_remove {
                        // Remove job from the processing set
                        let _: i32 = conn.srem(&processing_key, &job_id).await?;

                        // Delete the job key
                        let _: () = conn.del(&job_key).await?;
                    }
                }
            }
        }
    }

    // Process standalone job keys (completed, failed, or cancelled jobs)
    for job_key in job_keys {
        let job_json: Option<String> = conn.get(&job_key).await?;

        if let Some(json) = job_json {
            if let Ok(job) = Job::from_json(&json) {
                // Check if the job should be removed based on age and status
                let should_remove = job.created_at.is_some_and(|created_at| {
                    created_at < cutoff_date && status.map_or(true, |s| s.contains(&job.status))
                });

                if should_remove {
                    // Delete the job key
                    let _: () = conn.del(&job_key).await?;
                }
            }
        }
    }

    Ok(())
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
        let processing_jobs: Vec<String> = conn.smembers(&processing_key).await?;

        // Check each job in the processing set
        for job_id in &processing_jobs {
            // Get the actual job data using the ID
            let job_key = String::from(JOB_KEY_PREFIX) + job_id;
            let job_json: Option<String> = conn.get(&job_key).await?;

            if let Some(json) = job_json {
                if let Ok(mut job) = Job::from_json(&json) {
                    // Check if the job is old enough to be requeued
                    let should_requeue = if let Some(updated_at) = job.updated_at {
                        updated_at < cutoff_time
                    } else if let Some(created_at) = job.created_at {
                        // If no updated_at, use created_at
                        created_at < cutoff_time
                    } else {
                        false
                    };

                    if should_requeue {
                        // Job has been processing for too long, requeue it
                        job.status = JobStatus::Queued;
                        job.updated_at = Some(Utc::now());

                        // Update the job in Redis
                        if let Ok(updated_json) = job.to_json() {
                            // First, remove from the processing set
                            let _: i32 = conn.srem(&processing_key, job_id).await?;

                            // Update the job record
                            let _: () = conn.set(&job_key, &updated_json).await?;

                            // Add back to the queue
                            let _: () = conn.rpush(&queue_key, &updated_json).await?;

                            // Track count for logging
                            *requeued_counts.entry(queue_name.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    // Also check for failed jobs to requeue
    let failed_pattern = "failed:*";
    let failed_keys: Vec<String> = redis::cmd("KEYS")
        .arg(failed_pattern)
        .query_async(&mut conn)
        .await?;

    for failed_key in failed_keys {
        // Extract queue name from failed key
        let queue_name = failed_key.trim_start_matches("failed:").to_string();
        let queue_key = format!("{QUEUE_KEY_PREFIX}{queue_name}");

        // Get all jobs in the failed set
        let failed_jobs: Vec<String> = conn.smembers(&failed_key).await?;

        // Check each job in the failed set
        for job_id in &failed_jobs {
            // Get the actual job data using the ID
            let job_key = String::from(JOB_KEY_PREFIX) + job_id;
            let job_json: Option<String> = conn.get(&job_key).await?;

            if let Some(json) = job_json {
                if let Ok(mut job) = Job::from_json(&json) {
                    // Check if the job is old enough to be requeued
                    let should_requeue = if let Some(updated_at) = job.updated_at {
                        updated_at < cutoff_time && job.status == JobStatus::Failed
                    } else {
                        false
                    };

                    if should_requeue {
                        // Job has been failed for long enough, requeue it
                        job.status = JobStatus::Queued;
                        job.updated_at = Some(Utc::now());

                        // Update the job in Redis
                        if let Ok(updated_json) = job.to_json() {
                            // First, remove from the failed set
                            let _: i32 = conn.srem(&failed_key, job_id).await?;

                            // Update the job record
                            let _: () = conn.set(&job_key, &updated_json).await?;

                            // Add back to the queue
                            let _: () = conn.rpush(&queue_key, &updated_json).await?;

                            // Track count for logging
                            *requeued_counts.entry(queue_name.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    // Log the requeue counts
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
        let queue_jobs: Vec<String> = conn.lrange(&queue_key, 0, -1).await?;

        // Process each job individually
        for job_json in queue_jobs {
            if let Ok(mut job) = Job::from_json(&job_json) {
                if job.name == job_name && job.status == JobStatus::Queued {
                    // Mark this job as cancelled
                    job.status = JobStatus::Cancelled;
                    job.updated_at = Some(Utc::now());

                    // Update the job key
                    let job_key = String::from(JOB_KEY_PREFIX) + &job.id;
                    let updated_json = job.to_json()?;

                    // Remove this specific job from the queue
                    let _: i32 = conn.lrem(&queue_key, 1, &job_json).await?;

                    // Update the job in Redis
                    let _: () = conn.set(&job_key, &updated_json).await?;

                    // Store cancelled job in a set for tracking (optional)
                    let cancelled_key = format!(
                        "cancelled:{}",
                        queue_key.trim_start_matches(QUEUE_KEY_PREFIX)
                    );
                    let _: () = conn.sadd(&cancelled_key, &updated_json).await?;
                }
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

    // add if missing
    if let Some(config_queues) = config_queues {
        for q in config_queues {
            if !queues.iter().any(|aq| q == aq) {
                queues.push(q.to_string());
            }
        }
    }

    queues
}

pub struct RunOpts {
    pub num_workers: u32,
    pub poll_interval_sec: u32,
    pub queues: Option<Vec<String>>,
}

/// Create this provider
///
/// # Errors
///
/// This function will return an error if it fails
#[allow(clippy::unused_async)]
pub async fn create_provider(qcfg: &RedisQueueConfig) -> Result<Queue> {
    let client = connect(&qcfg.uri)?;
    let registry = JobRegistry::new();
    let token = CancellationToken::new();

    let run_opts = RunOpts {
        num_workers: qcfg.num_workers,
        poll_interval_sec: 1,
        queues: qcfg.queues.clone(),
    };

    debug!(
        queues = ?qcfg.queues,
        num_workers = qcfg.num_workers,
        "creating Redis queue provider"
    );

    // Wait for 3 seconds to ensure Redis is ready
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    Ok(Queue::Redis(
        client,
        Arc::new(tokio::sync::Mutex::new(registry)),
        run_opts,
        token,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg::redis::setup_redis_container;
    use chrono::Utc;
    use testcontainers::{ContainerAsync, GenericImage};

    async fn setup_redis() -> (RedisPool, ContainerAsync<GenericImage>) {
        let (redis_url, container) = setup_redis_container().await;

        // Connect to Redis
        let client = connect(&redis_url).expect("connect to redis");

        (client, container)
    }

    async fn redis_seed_data(client: &RedisPool) -> Result<()> {
        // Creating processed jobs
        let now = Utc::now();
        for i in 0..5 {
            let complete_job = Job {
                id: format!("job{}", i),
                name: "TestJob".to_string(),
                data: serde_json::json!({"counter": i}),
                status: JobStatus::Completed,
                run_at: now,
                interval: None,
                created_at: Some(now - chrono::Duration::days(15)),
                updated_at: Some(now - chrono::Duration::days(15)),
                tags: None,
            };

            let mut conn = get_connection(client).await?;
            // Store job data
            let _: () = conn
                .set(format!("{JOB_KEY_PREFIX}job{i}"), complete_job.to_json()?)
                .await?;
        }

        // Create queued jobs
        let args = serde_json::json!({"hello": "world"});
        enqueue(client, "TestJob".to_string(), None, args, None).await?;

        // Create job with tags
        let args = serde_json::json!({"hello": "tagged"});
        enqueue(
            client,
            "TaggedJob".to_string(),
            None,
            args,
            Some(vec!["important".to_string(), "urgent".to_string()]),
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
        let job_opt = dequeue(&client, &queues, &[]).await.expect("dequeue");

        // Verify job was dequeued
        assert!(job_opt.is_some());
    }

    #[tokio::test]
    async fn test_can_clear_redis() {
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Seed data
        if let Err(e) = redis_seed_data(&client).await {
            panic!("Failed to seed data: {}", e);
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
            enqueue(&client, "PasswordReset".to_string(), None, args, None)
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
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Test enqueue with custom queue
        let args = serde_json::json!({"email": "user@example.com"});
        assert!(enqueue(
            &client,
            "EmailNotification".to_string(),
            Some("mailer".to_string()),
            args,
            None
        )
        .await
        .is_ok());

        // Verify job was created in correct queue first
        let mut conn = get_connection(&client).await.expect("get connection");
        let queue_key = format!("{QUEUE_KEY_PREFIX}mailer");
        let queue_len: i64 = conn.llen(&queue_key).await.expect("get queue length");
        assert_eq!(queue_len, 1);

        // Test dequeue from mailer queue
        let queues = vec!["mailer".to_string()];
        let _job_opt = dequeue(&client, &queues, &[]).await.expect("dequeue");

        // Queue should now be empty
        let queue_len: i64 = conn.llen(&queue_key).await.expect("get queue length");
        assert_eq!(queue_len, 0);
    }

    #[tokio::test]
    async fn test_can_complete_job_redis() {
        // Setup Redis directly with reliable container setup
        // Setup Redis directly
        let (client, _container) = setup_redis().await;

        // Add job
        let args = serde_json::json!({"task": "test"});
        assert!(enqueue(&client, "TestJob".to_string(), None, args, None)
            .await
            .is_ok());

        // Dequeue job
        let queues = vec!["default".to_string()];
        let job_opt = dequeue(&client, &queues, &[]).await.expect("dequeue");
        let (job, queue) = job_opt.unwrap();

        // Complete job
        assert!(complete_job(&client, &job.id, &queue, None).await.is_ok());

        // Verify job is not in processing set
        let mut conn = get_connection(&client).await.expect("get connection");
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
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Add job
        let args = serde_json::json!({"task": "recurring"});
        assert!(
            enqueue(&client, "RecurringJob".to_string(), None, args, None)
                .await
                .is_ok()
        );

        // Dequeue job
        let queues = vec!["default".to_string()];
        let job_opt = dequeue(&client, &queues, &[]).await.expect("dequeue");
        let (job, queue) = job_opt.unwrap();

        // Complete job with interval to reschedule
        assert!(complete_job(&client, &job.id, &queue, Some(1000))
            .await
            .is_ok());

        // Verify job is back in queue
        let mut conn = get_connection(&client).await.expect("get connection");
        let queue_key = format!("{QUEUE_KEY_PREFIX}{queue}");
        let queue_len: i64 = conn.llen(&queue_key).await.expect("get queue length");
        assert_eq!(queue_len, 1);

        // Verify job has future run_at time
        let queue_jobs: Vec<String> = conn
            .lrange(&queue_key, 0, -1)
            .await
            .expect("get queue jobs");
        let requeued_job = Job::from_json(&queue_jobs[0]).expect("parse job");
        assert!(requeued_job.run_at > Utc::now());
    }

    #[tokio::test]
    async fn test_can_fail_job_redis() {
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Add job
        let args = serde_json::json!({"task": "test"});
        assert!(enqueue(&client, "TestJob".to_string(), None, args, None)
            .await
            .is_ok());

        // Dequeue job
        let queues = vec!["default".to_string()];
        let job_opt = dequeue(&client, &queues, &[]).await.expect("dequeue");
        let (job, queue) = job_opt.unwrap();

        // Fail job
        let error = Error::string("test failure");
        assert!(fail_job(&client, &job.id, &queue, &error).await.is_ok());

        // Verify job is not in processing set
        let mut conn = get_connection(&client).await.expect("get connection");
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

    #[tokio::test]
    async fn test_can_get_jobs_redis() {
        // Setup Redis directly with testcontainer
        let (client, _container) = setup_redis().await;

        // Seed data
        redis_seed_data(&client).await.expect("seed data");

        // Get all jobs
        let all_jobs = get_jobs(&client, None, None).await.expect("get all jobs");
        assert!(!all_jobs.is_empty());

        // Get jobs by status
        let queued_jobs = get_jobs(&client, Some(&vec![JobStatus::Queued]), None)
            .await
            .expect("get queued jobs");
        for job in &queued_jobs {
            assert_eq!(job.status, JobStatus::Queued);
        }

        let failed_jobs = get_jobs(&client, Some(&vec![JobStatus::Failed]), None)
            .await
            .expect("get failed jobs");
        for job in &failed_jobs {
            assert_eq!(job.status, JobStatus::Failed);
        }

        // Verify combined status filter
        let combined_jobs = get_jobs(
            &client,
            Some(&vec![JobStatus::Completed, JobStatus::Failed]),
            None,
        )
        .await
        .expect("get combined jobs");
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
        assert!(registry
            .register_worker("TestJob".to_string(), TestWorker)
            .is_ok());

        // Add job
        let args = serde_json::json!("test args");
        assert!(enqueue(&client, "TestJob".to_string(), None, args, None)
            .await
            .is_ok());

        // Run registry with worker for a short time
        let opts = RunOpts {
            num_workers: 1,
            poll_interval_sec: 1,
            queues: None,
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
        // Use the proper container setup pattern
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
            Some(vec!["tag1".to_string(), "common".to_string()])
        )
        .await
        .is_ok());

        let args2 = serde_json::json!({"task": "task2"});
        assert!(enqueue(
            &client,
            "TaggedJob".to_string(),
            Some("default".to_string()),
            args2,
            Some(vec!["tag2".to_string(), "common".to_string()])
        )
        .await
        .is_ok());

        let args3 = serde_json::json!({"task": "task3"});
        assert!(enqueue(
            &client,
            "TaggedJob".to_string(),
            Some("default".to_string()),
            args3,
            Some(vec!["tag3".to_string()])
        )
        .await
        .is_ok());

        // Test dequeue with tag1 filter
        let queues = vec!["default".to_string()];
        let job_opt = dequeue(&client, &queues, &["tag1".to_string()])
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
            Ok(_) => (),
            Err(e) => panic!("Failed to seed data: {}", e),
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
}
