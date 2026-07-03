/// Shared SQL-backed background job queue primitives.
///
/// This module holds the code that is identical between the Postgres
/// (`pg.rs`) and `SQLite` (`sqlt.rs`) queue providers: the `Job` model, the
/// `JobRegistry` (worker registration + run loop), and `RunOpts`. Each
/// backend supplies its own pool type and the three DB-coupled operations
/// (`dequeue`/`complete_job`/`fail_job`) through the [`Driver`] trait.
use std::{
    collections::HashMap, future::Future, panic::AssertUnwindSafe, pin::Pin, sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};

use super::{BackgroundWorker, JobStatus};
use crate::{Error, Result};

pub type JobId = String;
pub type JobData = JsonValue;

pub(crate) type JobHandler = Box<
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
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug)]
pub struct RunOpts {
    pub num_workers: u32,
    pub poll_interval_sec: u32,
}

/// Abstracts the backend-coupled operations of the SQL-based job queue
/// providers (Postgres, `SQLite`) so that [`JobRegistry::run`] can be shared
/// between them.
pub trait Driver: Send + Sync + 'static {
    type Pool: Clone + Send + Sync + 'static;

    fn idle_count(pool: &Self::Pool) -> usize;

    fn dequeue(
        pool: &Self::Pool,
        tags: &[String],
    ) -> impl std::future::Future<Output = crate::Result<Option<Job>>> + Send;

    fn complete_job(
        pool: &Self::Pool,
        id: &JobId,
        interval: Option<i64>,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;

    fn fail_job(
        pool: &Self::Pool,
        id: &JobId,
        error: &crate::Error,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;
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
    pub fn run<D: Driver>(
        &self,
        pool: &D::Pool,
        opts: &RunOpts,
        token: &CancellationToken,
        tags: &[String],
    ) -> Vec<JoinHandle<()>> {
        let mut jobs = Vec::new();

        let interval = opts.poll_interval_sec;
        for idx in 0..opts.num_workers {
            let handlers = self.handlers.clone();
            let worker_token = token.clone(); // Clone token for this worker
            let worker_tags = tags.to_vec();

            let pool = pool.clone();
            let job = tokio::spawn(async move {
                loop {
                    // Check for cancellation before potentially blocking on dequeue
                    if worker_token.is_cancelled() {
                        trace!(worker_id = idx, "Cancellation received, stopping worker");
                        break;
                    }
                    trace!(
                        pool_size = D::idle_count(&pool),
                        worker_id = idx,
                        "Connection pool stats"
                    );
                    let job_opt = match D::dequeue(&pool, &worker_tags).await {
                        Ok(t) => t,
                        Err(err) => {
                            error!(error = %err, "Failed to fetch job from queue");
                            None
                        }
                    };

                    if let Some(job) = job_opt {
                        debug!(job_id = %job.id, job_name = %job.name, "Processing job");
                        if let Some(handler) = handlers.get(&job.name) {
                            match handler(job.id.clone(), job.data.clone()).await {
                                Ok(()) => match D::complete_job(&pool, &job.id, job.interval).await
                                {
                                    Err(err) => {
                                        error!(
                                            error = %err,
                                            job_id = %job.id,
                                            job_name = %job.name,
                                            "Failed to mark job as completed"
                                        );
                                    }
                                    _ => {
                                        debug!(job_id = %job.id, "Job completed successfully");
                                    }
                                },
                                Err(err) => match D::fail_job(&pool, &job.id, &err).await {
                                    Err(fail_err) => {
                                        error!(
                                            error = %fail_err,
                                            job_id = %job.id,
                                            job_name = %job.name,
                                            "Failed to mark job as failed"
                                        );
                                    }
                                    _ => {
                                        debug!(job_id = %job.id, error = %err, "Job execution failed");
                                    }
                                },
                            }
                        } else {
                            error!(job_name = %job.name, "No handler registered for job");
                        }
                    } else {
                        // Use tokio::select! to wait for interval or cancellation
                        tokio::select! {
                            biased;
                            () = worker_token.cancelled() => {
                                trace!(worker_id = idx, "Cancellation received during sleep, stopping worker");
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
