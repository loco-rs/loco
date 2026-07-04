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

use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};

use super::BackgroundWorker;
pub use super::{Job, JobData, JobId};
use crate::{config::ReaperConfig, Error, Result};

/// Pings the job queue database by selecting a single row's `id` from the
/// given `table`, shared between the Postgres and `SQLite` backends (which
/// only differ in table name and pool/executor type).
///
/// # Errors
///
/// This function will return an error if it fails
pub(crate) async fn ping<'e, E>(executor: E, table: &str) -> Result<()>
where
    E: sqlx::Executor<'e>,
    <E::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<E::Database>,
{
    trace!("Pinging job queue database");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT id from {table} LIMIT 1"
    )))
    .execute(executor)
    .await?;
    Ok(())
}

/// Converts a database row into a [`Job`] object.
///
/// This function manually extracts the necessary fields from a row to populate a
/// [`Job`] object, shared between the Postgres and `SQLite` backends (their row
/// layouts and column types line up, so the mapping logic is identical).
///
/// **Note:** This function manually extracts values from the database row instead of using
/// the `FromRow` trait, which would require enabling the 'macros' feature in the dependencies.
/// The decision to avoid `FromRow` is made to keep the build smaller and faster, as the 'macros'
/// feature is unnecessary in the current dependency tree.
pub(crate) fn to_job<R>(row: &R) -> Result<Job>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> serde_json::Value: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<serde_json::Value>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> chrono::DateTime<chrono::Utc>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<chrono::DateTime<chrono::Utc>>:
        sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<i64>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> i32: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let tags_json: Option<serde_json::Value> = row.try_get("tags").unwrap_or_default();
    let tags = tags_json.and_then(|json_val| {
        if json_val.is_array() {
            let tags_vec: Vec<String> =
                serde_json::from_value(json_val).unwrap_or_else(|_| Vec::new());
            if tags_vec.is_empty() {
                None
            } else {
                Some(tags_vec)
            }
        } else {
            None
        }
    });

    Ok(Job {
        id: row.get("id"),
        name: row.get("name"),
        data: row.get("task_data"),
        status: row.get::<String, _>("status").parse().map_err(|err| {
            let status: String = row.get("status");
            tracing::error!(status, err = %err, "Unsupported job status in database");
            Error::string("invalid job status")
        })?,
        run_at: row.get("run_at"),
        interval: row.get("interval"),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
        tags,
        priority: row.get("priority"),
    })
}

pub(crate) type JobHandler = Box<
    dyn Fn(
            JobId,
            JobData,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<(), crate::Error>> + Send>>
        + Send
        + Sync,
>;

#[derive(Debug)]
pub struct RunOpts {
    pub num_workers: u32,
    pub poll_interval_sec: u32,
    /// Opt-in visibility-timeout reaper settings, populated from the queue
    /// config. `None` disables the reaper (default, backward-compatible).
    pub reaper: Option<ReaperConfig>,
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
