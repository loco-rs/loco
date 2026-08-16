use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
#[cfg(feature = "cli")]
use clap::ValueEnum;
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_variant::to_variant_name;
#[cfg(feature = "worker")]
pub mod pg;
#[cfg(feature = "worker_redis")]
pub mod redis;
#[cfg(feature = "worker")]
pub(crate) mod sql;
#[cfg(feature = "worker")]
pub mod sqlt;

use crate::{
    app::AppContext,
    config::{
        self, Config, PostgresQueueConfig, QueueConfig, RedisQueueConfig, SqliteQueueConfig,
        WorkerMode,
    },
    Error, Result,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[non_exhaustive]
pub enum JobStatus {
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

impl std::str::FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(Self::Queued),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("Invalid status: {s}")),
        }
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_variant_name(self).expect("only enum supported").fmt(f)
    }
}

pub type JobId = String;
pub type JobData = JsonValue;

/// A background job, shared between the SQL-based (Postgres/`SQLite`) and
/// Redis queue providers.
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

/// A boxed, type-erased job handler: takes a job's id and data and runs the
/// worker's `perform`, shared between every queue backend's registry.
pub type JobHandler = Box<
    dyn Fn(
            JobId,
            JobData,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), crate::Error>> + Send>>
        + Send
        + Sync,
>;

/// Erases a [`BackgroundWorker`]'s argument type into a [`JobHandler`] that
/// can be stored in a backend's job registry alongside handlers for other,
/// differently-typed workers.
///
/// This closure body used to be duplicated byte-for-byte between the
/// Postgres/`SQLite` registry (`sql.rs`) and the Redis registry (`redis.rs`);
/// it now lives here once.
pub(crate) fn erase_worker<Args, W>(worker: W) -> JobHandler
where
    Args: Send + Serialize + Sync + 'static + for<'de> Deserialize<'de>,
    W: BackgroundWorker<Args> + 'static,
{
    let worker = Arc::new(worker);
    Box::new(move |_job_id: JobId, job_data: JobData| {
        let w = worker.clone();
        Box::pin(async move {
            let args = serde_json::from_value::<Args>(job_data);
            match args {
                Ok(args) => {
                    // Wrap the perform call in catch_unwind to handle panics
                    match std::panic::AssertUnwindSafe(w.perform(args))
                        .catch_unwind()
                        .await
                    {
                        Ok(result) => result,
                        Err(panic) => {
                            let panic_msg = panic
                                .downcast_ref::<String>()
                                .map(String::as_str)
                                .or_else(|| panic.downcast_ref::<&str>().copied())
                                .unwrap_or("Unknown panic occurred");
                            tracing::error!(err = panic_msg, "worker panicked");
                            Err(Error::string(panic_msg))
                        }
                    }
                }
                Err(err) => Err(err.into()),
            }
        })
            as std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), crate::Error>> + Send>>
    })
}

/// Object-safe interface implemented by every queue backend (Postgres,
/// `SQLite`, Redis, and the built-in no-op).
///
/// [`Queue`] is a thin newtype wrapper around `Arc<dyn QueueProvider>` that
/// forwards every call here.
#[async_trait]
pub trait QueueProvider: Send + Sync {
    /// Add a job to the queue. See [`Queue::enqueue`] for the full contract.
    ///
    /// # Errors
    /// This function will return an error if the enqueue operation fails.
    async fn enqueue(
        &self,
        class: String,
        queue: Option<String>,
        args: JsonValue,
        tags: Option<Vec<String>>,
        priority: Option<i32>,
    ) -> Result<Option<String>>;

    /// Registers a pre-erased job handler under `name`.
    ///
    /// # Errors
    /// This function will return an error if it fails to register.
    async fn register_handler(&self, name: String, handler: JobHandler) -> Result<()>;

    /// Runs the worker loop for this provider.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn run(&self, tags: Vec<String>) -> Result<()>;

    /// Runs the setup of this provider.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn setup(&self) -> Result<()>;

    /// Clears all jobs from this provider.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn clear(&self) -> Result<()>;

    /// Pings this provider.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn ping(&self) -> Result<()>;

    /// Retrieves jobs from this provider, optionally filtered.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn get_jobs(
        &self,
        status: Option<&Vec<JobStatus>>,
        age_days: Option<i64>,
    ) -> Result<Vec<Job>>;

    /// Cancels queued jobs by name.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn cancel_jobs_by_name(&self, name: &str) -> Result<()>;

    /// Clears jobs matching any of the given statuses.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn clear_by_status(&self, status: Vec<JobStatus>) -> Result<()>;

    /// Clears jobs older than `age_days`, optionally filtered by status.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn clear_jobs_older_than(
        &self,
        age_days: i64,
        status: Option<&Vec<JobStatus>>,
    ) -> Result<()>;

    /// Requeues jobs stuck in processing for longer than `age_minutes`.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn requeue(&self, age_minutes: &i64) -> Result<()>;

    /// Moves failed jobs back to [`JobStatus::Queued`], returning how many
    /// moved. With `id`, only that job; without, every failed job.
    ///
    /// Distinct from [`Queue::requeue`], which rescues jobs stranded in
    /// [`JobStatus::Processing`] by a crashed worker and cannot touch a failed
    /// one. Without this, a failed job is terminal: there is no automatic retry
    /// in any driver, so an operator's only recourse was to dump and re-import.
    ///
    /// # Errors
    /// This function will return an error if it fails.
    async fn retry_failed(&self, id: Option<&str>) -> Result<u64>;

    /// A short, human-readable description of this provider.
    fn describe(&self) -> String;

    /// Signals this provider's worker loop to stop.
    ///
    /// # Errors
    /// Does not currently return an error, but implementations might, so
    /// using Result here as return type.
    fn shutdown(&self) -> Result<()>;
}

/// Process worker task handles and handle any errors.
///
/// Shared by every backend's [`QueueProvider::run`] implementation.
///
/// # Errors
/// This function will return an error if a worker task fails to join
#[cfg(feature = "worker")]
pub(crate) async fn process_worker_handles(
    handles: Vec<tokio::task::JoinHandle<()>>,
) -> Result<()> {
    let handle_count = handles.len();
    tracing::debug!(worker_count = handle_count, "Processing worker handles");

    for (index, handle) in handles.into_iter().enumerate() {
        if let Err(e) = handle.await {
            if e.is_cancelled() {
                tracing::debug!(
                    worker_index = index,
                    "Worker task cancelled during shutdown"
                );
            } else if e.is_panic() {
                tracing::error!(worker_index = index, "Worker task panicked");
                std::panic::resume_unwind(e.into_panic());
            } else {
                tracing::error!(worker_index = index, error = ?e, "Worker task failed to join");
                return Err(crate::Error::Worker(format!("Worker join error: {e}")));
            }
        }
    }
    tracing::info!(
        worker_count = handle_count,
        "All worker tasks finished successfully"
    );
    Ok(())
}

/// No-op queue provider used when no queue backend is configured (or none
/// was compiled in). Reproduces the old `Queue` enum's `None`/`_` match-arm
/// behavior exactly, method for method.
struct NoopQueue;

#[async_trait]
impl QueueProvider for NoopQueue {
    async fn enqueue(
        &self,
        _class: String,
        _queue: Option<String>,
        _args: JsonValue,
        _tags: Option<Vec<String>>,
        _priority: Option<i32>,
    ) -> Result<Option<String>> {
        Ok(None)
    }

    async fn register_handler(&self, _name: String, _handler: JobHandler) -> Result<()> {
        Ok(())
    }

    async fn run(&self, _tags: Vec<String>) -> Result<()> {
        tracing::error!(
            "No queue provider is configured: compile with at least one queue provider feature"
        );
        Ok(())
    }

    async fn setup(&self) -> Result<()> {
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        Ok(())
    }

    async fn ping(&self) -> Result<()> {
        Ok(())
    }

    async fn get_jobs(
        &self,
        _status: Option<&Vec<JobStatus>>,
        _age_days: Option<i64>,
    ) -> Result<Vec<Job>> {
        tracing::error!(
            "No queue provider is configured: compile with at least one queue provider feature"
        );
        Err(Error::string("provider not configured"))
    }

    async fn cancel_jobs_by_name(&self, _name: &str) -> Result<()> {
        tracing::error!(
            "No queue provider is configured: compile with at least one queue provider feature"
        );
        Err(Error::string("provider not configured"))
    }

    async fn clear_by_status(&self, _status: Vec<JobStatus>) -> Result<()> {
        tracing::error!(
            "No queue provider is configured: compile with at least one queue provider feature"
        );
        Err(Error::string("provider not configured"))
    }

    async fn clear_jobs_older_than(
        &self,
        _age_days: i64,
        _status: Option<&Vec<JobStatus>>,
    ) -> Result<()> {
        tracing::error!(
            "No queue provider is configured: compile with at least one queue provider feature"
        );
        Err(Error::string("provider not configured"))
    }

    async fn requeue(&self, _age_minutes: &i64) -> Result<()> {
        tracing::error!(
            "No queue provider is configured: compile with at least one queue provider feature"
        );
        Err(Error::string("provider not configured"))
    }

    async fn retry_failed(&self, _id: Option<&str>) -> Result<u64> {
        tracing::error!(
            "No queue provider is configured: compile with at least one queue provider feature"
        );
        Err(Error::string("provider not configured"))
    }

    fn describe(&self) -> String {
        "no queue".to_string()
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// A handle to a background job queue.
///
/// Newtype facade over `Arc<dyn QueueProvider>`: every operation is a thin
/// delegate to the configured provider (Postgres, `SQLite`, Redis, or
/// [`Queue::empty`]'s no-op).
pub struct Queue(Arc<dyn QueueProvider>);

impl Queue {
    /// Wraps a third-party (or built-in) [`QueueProvider`] as a [`Queue`].
    #[must_use]
    pub fn from_provider(provider: Arc<dyn QueueProvider>) -> Self {
        Self(provider)
    }

    /// A [`Queue`] with no configured provider. Replaces the old
    /// `Queue::None` enum variant.
    #[must_use]
    pub fn empty() -> Self {
        Self(Arc::new(NoopQueue))
    }

    /// Add a job to the queue.
    ///
    /// Returns the job ID when the configured provider supports it
    /// (`Some(String)` for Redis / `PostgreSQL` / `SQLite`), or `None` when no
    /// queue provider is configured.
    ///
    /// # Errors
    ///
    /// This function will return an error if the enqueue operation fails.
    ///
    /// Priority semantics for all queue backends:
    /// - Higher value means higher urgency.
    /// - Valid range is full `i32` (`-2_147_483_648..=2_147_483_647`).
    /// - Ties are resolved by earlier `run_at`, then by stable job id ordering.
    pub async fn enqueue<A: Serialize + Send + Sync>(
        &self,
        class: String,
        queue: Option<String>,
        args: A,
        tags: Option<Vec<String>>,
        priority: Option<i32>,
    ) -> Result<Option<String>> {
        tracing::debug!(worker = class, queue = ?queue, tags = ?tags, "Enqueuing background job");
        self.0
            .enqueue(class, queue, serde_json::to_value(args)?, tags, priority)
            .await
    }

    /// Register a worker
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn register<
        A: Serialize + Send + Sync + 'static + for<'de> serde::Deserialize<'de>,
        W: BackgroundWorker<A> + 'static,
    >(
        &self,
        worker: W,
    ) -> Result<()> {
        tracing::info!(worker = W::class_name(), "Registering background worker");
        let name = W::class_name();
        let handler = erase_worker(worker);
        self.0.register_handler(name, handler).await
    }

    /// Runs the worker loop for this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn run(&self, tags: Vec<String>) -> Result<()> {
        tracing::info!("Starting background job processing");
        self.0.run(tags).await
    }

    /// Runs the setup of this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn setup(&self) -> Result<()> {
        self.0.setup().await
    }

    /// Performs clear on this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn clear(&self) -> Result<()> {
        tracing::info!("Clearing all jobs from queue");
        self.0.clear().await
    }

    /// Returns a ping of this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn ping(&self) -> Result<()> {
        tracing::trace!("Pinging job queue");
        self.0.ping().await
    }

    #[must_use]
    pub fn describe(&self) -> String {
        self.0.describe()
    }

    /// # Errors
    ///
    /// Does not currently return an error, but the postgres or other future
    /// queue implementations might, so using Result here as return type.
    pub fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down background job processing");
        self.0.shutdown()
    }

    async fn get_jobs(
        &self,
        status: Option<&Vec<JobStatus>>,
        age_days: Option<i64>,
    ) -> Result<Vec<Job>> {
        tracing::info!(status = ?status, age_days = ?age_days, "Retrieving jobs");
        self.0.get_jobs(status, age_days).await
    }

    /// Cancels jobs based on the given job name for the configured queue provider.
    ///
    /// # Errors
    /// - If no queue provider is configured, it will return an error indicating the lack of configuration.
    /// - If the Redis provider is selected, it will return an error stating that cancellation is not supported.
    /// - Any error in the underlying provider's cancellation logic will propagate from the respective function.
    ///
    pub async fn cancel_jobs(&self, job_name: &str) -> Result<()> {
        tracing::info!(job_name = job_name, "Cancelling jobs by name");
        self.0.cancel_jobs_by_name(job_name).await
    }

    /// Clears jobs older than a specified number of days for the configured queue provider.
    ///
    /// # Errors
    /// - If no queue provider is configured, it will return an error indicating the lack of configuration.
    /// - If the Redis provider is selected, it will return an error stating that clearing jobs is not supported.
    /// - Any error in the underlying provider's job clearing logic will propagate from the respective function.
    ///
    pub async fn clear_jobs_older_than(
        &self,
        age_days: i64,
        status: &Vec<JobStatus>,
    ) -> Result<()> {
        tracing::info!(age_days = age_days, status = ?status, "Clearing older jobs");
        self.0.clear_jobs_older_than(age_days, Some(status)).await
    }

    /// Clears jobs based on their status for the configured queue provider.
    ///
    /// # Errors
    /// - If no queue provider is configured, it will return an error indicating the lack of configuration.
    /// - If the Redis provider is selected, it will return an error stating that clearing jobs is not supported.
    /// - Any error in the underlying provider's job clearing logic will propagate from the respective function.
    pub async fn clear_by_status(&self, status: Vec<JobStatus>) -> Result<()> {
        tracing::info!(status = ?status, "Clearing jobs by status");
        self.0.clear_by_status(status).await
    }

    /// Requeued job with the given minutes ages.
    ///
    /// # Errors
    /// - If no queue provider is configured, it will return an error indicating the lack of configuration.
    /// - If the Redis provider is selected, it will return an error stating that clearing jobs is not supported.
    /// - Any error in the underlying provider's job clearing logic will propagate from the respective function.
    pub async fn requeue(&self, age_minutes: &i64) -> Result<()> {
        tracing::info!(age_minutes = age_minutes, "Requeuing stale jobs");
        self.0.requeue(age_minutes).await
    }

    /// Moves failed jobs back to [`JobStatus::Queued`], returning how many
    /// moved. With `id`, only that job; without, every failed job.
    ///
    /// # Errors
    /// - If no queue provider is configured, it will return an error indicating the lack of configuration.
    /// - Any error in the underlying provider's retry logic will propagate from the respective function.
    pub async fn retry_failed(&self, id: Option<&str>) -> Result<u64> {
        tracing::info!(job_id = ?id, "Retrying failed jobs");
        self.0.retry_failed(id).await
    }

    /// Dumps the list of jobs to a YAML file at the specified path.
    ///
    /// This function retrieves jobs from the queue, optionally filtered by their
    /// status, and writes the job data to a YAML file.
    ///
    /// # Errors
    /// - If the specified path cannot be created, an error will be returned.
    /// - If the job retrieval or YAML serialization fails, an error will be returned.
    /// - If there is an issue creating the dump file, an error will be returned
    pub async fn dump(
        &self,
        path: &Path,
        status: Option<&Vec<JobStatus>>,
        age_days: Option<i64>,
    ) -> Result<PathBuf> {
        tracing::info!(path = %path.display(), status = ?status, age_days = ?age_days, "Dumping jobs to file");

        if !path.exists() {
            tracing::debug!(path = %path.display(), "Directory does not exist, creating...");
            std::fs::create_dir_all(path)?;
        }

        let dump_file = path.join(format!(
            "loco-dump-jobs-{}.yaml",
            chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S")
        ));

        let jobs = self.get_jobs(status, age_days).await?;

        let data = serde_yaml::to_string(&serde_json::to_value(jobs)?)?;
        let mut file = File::create(&dump_file)?;
        file.write_all(data.as_bytes())?;

        tracing::info!(file = %dump_file.display(), "Jobs successfully dumped to file");
        Ok(dump_file)
    }

    /// Imports jobs from a YAML file into the configured queue provider.
    ///
    /// This function reads job data from a YAML file located at the specified `path` and imports
    /// the jobs into the queue.
    ///
    /// # Errors
    /// - If there is an issue opening or reading the YAML file, an error will be returned.
    /// - If any issues occur while enqueuing the jobs, the function will return an error.
    pub async fn import(&self, path: &Path) -> Result<()> {
        tracing::info!(path = %path.display(), "Importing jobs from file");

        // Read the file asynchronously rather than blocking the runtime on a
        // synchronous `File`/`from_reader`.
        let import_bytes = tokio::fs::read(path).await?;
        let jobs: Vec<Job> = serde_yaml::from_slice(&import_bytes)?;
        for job in jobs {
            self.enqueue(
                job.name.clone(),
                None,
                job.data,
                job.tags.clone(),
                Some(job.priority),
            )
            .await?;
        }

        Ok(())
    }
}

#[async_trait]
pub trait BackgroundWorker<A: Send + Sync + serde::Serialize + 'static>: Send + Sync {
    /// If you have a specific queue
    /// in mind and the provider supports custom / priority queues, make your
    /// worker return it. Otherwise, return `None`.
    #[must_use]
    fn queue() -> Option<String> {
        None
    }

    /// Specifies tags associated with this worker. Workers might only process jobs
    /// matching specific tags during startup.
    #[must_use]
    fn tags() -> Vec<String> {
        Vec::new()
    }

    fn build(ctx: &AppContext) -> Self;
    #[must_use]
    fn class_name() -> String
    where
        Self: Sized,
    {
        use heck::ToUpperCamelCase;
        let type_name = std::any::type_name::<Self>();
        let name = type_name.split("::").last().unwrap_or(type_name);
        name.to_upper_camel_case()
    }
    /// Enqueue (or run) the job at the default priority and return its ID.
    ///
    /// Convenience wrapper over [`BackgroundWorker::perform_later_with_priority`]
    /// with no explicit priority.
    async fn perform_later(ctx: &AppContext, args: A) -> crate::Result<String>
    where
        Self: Sized,
    {
        Self::perform_later_with_priority(ctx, args, None).await
    }

    /// Enqueue (or run) the job with an explicit priority and return its ID.
    ///
    /// In `BackgroundQueue` mode the ID is the one assigned by the queue
    /// provider. In foreground/async modes (or when no provider is configured)
    /// a fresh ID is generated so callers always get a stable handle back.
    /// Higher `priority` values are dequeued first (see [`Queue::enqueue`]).
    async fn perform_later_with_priority(
        ctx: &AppContext,
        args: A,
        priority: Option<i32>,
    ) -> crate::Result<String>
    where
        Self: Sized,
    {
        let job_id = match &ctx.config.workers.mode {
            WorkerMode::BackgroundQueue => {
                if let Some(p) = &ctx.queue_provider {
                    let tags = Self::tags();
                    let tags_option = if tags.is_empty() { None } else { Some(tags) };
                    p.enqueue(
                        Self::class_name(),
                        Self::queue(),
                        args,
                        tags_option,
                        priority,
                    )
                    .await?
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                } else {
                    tracing::error!(
                        "perform_later: background queue is selected, but queue was not populated \
                         in context"
                    );
                    uuid::Uuid::new_v4().to_string()
                }
            }
            WorkerMode::ForegroundBlocking => {
                Self::build(ctx).perform(args).await?;
                uuid::Uuid::new_v4().to_string()
            }
            WorkerMode::BackgroundAsync => {
                let dx = ctx.clone();
                tokio::spawn(async move {
                    if let Err(err) = Self::build(&dx).perform(args).await {
                        tracing::error!(err = err.to_string(), "worker failed to perform job");
                    }
                });
                uuid::Uuid::new_v4().to_string()
            }
        };
        Ok(job_id)
    }

    async fn perform(&self, args: A) -> crate::Result<()>;
}

/// Initialize the system according to configuration
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn converge(queue: &Queue, config: &QueueConfig) -> Result<()> {
    queue.setup().await?;
    match config {
        QueueConfig::Postgres(PostgresQueueConfig {
            dangerously_flush,
            uri: _,
            max_connections: _,
            enable_logging: _,
            connect_timeout: _,
            idle_timeout: _,
            poll_interval_sec: _,
            num_workers: _,
            min_connections: _,
            reaper: _,
        })
        | QueueConfig::Sqlite(SqliteQueueConfig {
            dangerously_flush,
            uri: _,
            max_connections: _,
            enable_logging: _,
            connect_timeout: _,
            idle_timeout: _,
            poll_interval_sec: _,
            num_workers: _,
            min_connections: _,
            reaper: _,
        })
        | QueueConfig::Redis(RedisQueueConfig {
            dangerously_flush,
            uri: _,
            queues: _,
            num_workers: _,
            reaper: _,
        }) => {
            if *dangerously_flush {
                tracing::warn!("Flush mode enabled - clearing all jobs from queue");
                queue.clear().await?;
            }
        }
    }
    Ok(())
}

/// Create a provider
///
/// # Errors
///
/// This function will return an error if fails to build
#[allow(clippy::missing_panics_doc)]
pub async fn create_queue_provider(config: &Config) -> Result<Option<Arc<Queue>>> {
    if config.workers.mode == config::WorkerMode::BackgroundQueue {
        if let Some(queue) = &config.queue {
            match queue {
                #[cfg(feature = "worker_redis")]
                config::QueueConfig::Redis(qcfg) => {
                    tracing::debug!("Creating Redis queue provider");
                    Ok(Some(Arc::new(redis::create_provider(qcfg).await?)))
                }
                #[cfg(feature = "worker")]
                config::QueueConfig::Postgres(qcfg) => {
                    tracing::debug!("Creating Postgres queue provider");
                    Ok(Some(Arc::new(pg::create_provider(qcfg).await?)))
                }
                #[cfg(feature = "worker")]
                config::QueueConfig::Sqlite(qcfg) => {
                    tracing::debug!("Creating SQLite queue provider");
                    Ok(Some(Arc::new(sqlt::create_provider(qcfg).await?)))
                }

                #[allow(unreachable_patterns)]
                _ => Err(Error::string(
                    "No queue provider feature was selected and compiled, but queue configuration \
                     is present",
                )),
            }
        } else {
            // tracing::warn!("Worker mode is BackgroundQueue but no queue configuration is present");
            Ok(None)
        }
    } else {
        // tracing::debug!("Worker mode is not BackgroundQueue, skipping queue provider creation");
        Ok(None)
    }
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use insta::assert_debug_snapshot;

    use super::*;
    use crate::tests_cfg;

    fn sqlite_config(db_path: &Path) -> SqliteQueueConfig {
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

    #[tokio::test]
    async fn can_dump_jobs() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let qcfg = sqlite_config(tree_fs.root.as_path());
        let queue = sqlt::create_provider(&qcfg)
            .await
            .expect("create sqlite queue");

        let pool = sqlx::SqlitePool::connect(&qcfg.uri)
            .await
            .expect("connect to sqlite db");

        queue.setup().await.expect("setup sqlite db");
        tests_cfg::queue::sqlite_seed_data(&pool).await;

        let dump_file = queue
            .dump(
                tree_fs.root.as_path(),
                Some(&vec![JobStatus::Failed, JobStatus::Cancelled]),
                None,
            )
            .await
            .expect("dump jobs");

        assert_debug_snapshot!(std::fs::read_to_string(dump_file).unwrap());
    }

    #[tokio::test]
    async fn cat_import_jobs_form_file() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let qcfg = sqlite_config(tree_fs.root.as_path());
        let queue = sqlt::create_provider(&qcfg)
            .await
            .expect("create sqlite queue");

        let pool = sqlx::SqlitePool::connect(&qcfg.uri)
            .await
            .expect("connect to sqlite db");

        queue.setup().await.expect("setup sqlite db");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlt_loco_queue")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(count, 0);

        queue
            .import(
                PathBuf::from("tests")
                    .join("fixtures")
                    .join("queue")
                    .join("jobs.yaml")
                    .as_path(),
            )
            .await
            .expect("dump import");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlt_loco_queue")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(count, 14);
    }
}
