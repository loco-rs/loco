use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
#[cfg(feature = "cli")]
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;
#[cfg(feature = "bg_pg")]
pub mod pg;
#[cfg(feature = "bg_redis")]
pub mod redis;
#[cfg(feature = "bg_sqlt")]
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

// Queue struct now holds both a QueueProvider and QueueRegistrar
pub enum Queue {
    #[cfg(feature = "bg_redis")]
    Redis(
        redis::RedisPool,
        Arc<tokio::sync::Mutex<redis::JobRegistry>>,
        redis::RunOpts,
        tokio_util::sync::CancellationToken,
    ),
    #[cfg(feature = "bg_pg")]
    Postgres(
        pg::PgPool,
        std::sync::Arc<tokio::sync::Mutex<pg::JobRegistry>>,
        pg::RunOpts,
        tokio_util::sync::CancellationToken,
    ),
    #[cfg(feature = "bg_sqlt")]
    Sqlite(
        sqlt::SqlitePool,
        std::sync::Arc<tokio::sync::Mutex<sqlt::JobRegistry>>,
        sqlt::RunOpts,
        tokio_util::sync::CancellationToken,
    ),
    None,
}

impl Queue {
    /// Add a job to the queue
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    #[allow(unused_variables)]
    pub async fn enqueue<A: Serialize + Send + Sync>(
        &self,
        class: String,
        queue: Option<String>,
        args: A,
        tags: Option<Vec<String>>,
    ) -> Result<()> {
        tracing::debug!(worker = class, queue = ?queue, tags = ?tags, "Enqueuing background job");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _, _) => {
                redis::enqueue(pool, class, queue, args, tags).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => {
                pg::enqueue(
                    pool,
                    &class,
                    serde_json::to_value(args)?,
                    chrono::Utc::now(),
                    None,
                    tags,
                )
                .await
                .map_err(Box::from)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => {
                sqlt::enqueue(
                    pool,
                    &class,
                    serde_json::to_value(args)?,
                    chrono::Utc::now(),
                    None,
                    tags,
                )
                .await
                .map_err(Box::from)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Register a worker
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    #[allow(unused_variables)]
    pub async fn register<
        A: Serialize + Send + Sync + 'static + for<'de> serde::Deserialize<'de>,
        W: BackgroundWorker<A> + 'static,
    >(
        &self,
        worker: W,
    ) -> Result<()> {
        tracing::info!(worker = W::class_name(), "Registering background worker");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, p, _, _) => {
                let mut p = p.lock().await;
                p.register_worker(W::class_name(), worker)?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, registry, _, _) => {
                let mut r = registry.lock().await;
                r.register_worker(W::class_name(), worker)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(_, registry, _, _) => {
                let mut r = registry.lock().await;
                r.register_worker(W::class_name(), worker)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Runs the worker loop for this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    #[allow(unused_variables)]
    pub async fn run(&self, tags: Vec<String>) -> Result<()> {
        tracing::info!("Starting background job processing");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, registry, run_opts, token) => {
                let handles = registry
                    .lock()
                    .await
                    .run(pool, run_opts, &token.clone(), &tags);
                Self::process_worker_handles(handles).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, registry, run_opts, token) => {
                let handles = registry
                    .lock()
                    .await
                    .run(pool, run_opts, &token.clone(), &tags);
                Self::process_worker_handles(handles).await?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, registry, run_opts, token) => {
                let handles = registry
                    .lock()
                    .await
                    .run(pool, run_opts, &token.clone(), &tags);
                Self::process_worker_handles(handles).await?;
            }
            _ => {
                tracing::error!(
                    "No queue provider is configured: compile with at least one queue provider feature"
                );
            }
        }
        Ok(())
    }

    /// Process worker task handles and handle any errors
    ///
    /// # Errors
    /// This function will return an error if a worker task fails to join
    #[allow(dead_code)]
    async fn process_worker_handles(handles: Vec<tokio::task::JoinHandle<()>>) -> Result<()> {
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

    /// Runs the setup of this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn setup(&self) -> Result<()> {
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _, _) => {}
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => {
                pg::initialize_database(pool).await.map_err(Box::from)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => {
                sqlt::initialize_database(pool).await.map_err(Box::from)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Performs clear on this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn clear(&self) -> Result<()> {
        tracing::info!("Clearing all jobs from queue");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _, _) => {
                redis::clear(pool).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => {
                pg::clear(pool).await.map_err(Box::from)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => {
                sqlt::clear(pool).await.map_err(Box::from)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Returns a ping of this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn ping(&self) -> Result<()> {
        tracing::trace!("Pinging job queue");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _, _) => {
                redis::ping(pool).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => {
                pg::ping(pool).await.map_err(Box::from)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => {
                sqlt::ping(pool).await.map_err(Box::from)?;
            }
            _ => {}
        }
        Ok(())
    }

    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _, _) => "redis queue".to_string(),
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, _, _, _) => "postgres queue".to_string(),
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(_, _, _, _) => "sqlite queue".to_string(),
            _ => "no queue".to_string(),
        }
    }

    /// # Errors
    ///
    /// Does not currently return an error, but the postgres or other future
    /// queue implementations might, so using Result here as return type.
    pub fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down background job processing");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _, cancellation_token) => cancellation_token.cancel(),
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, _, _, cancellation_token) => cancellation_token.cancel(),
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(_, _, _, cancellation_token) => cancellation_token.cancel(),
            _ => {}
        }

        Ok(())
    }

    async fn get_jobs(
        &self,
        status: Option<&Vec<JobStatus>>,
        age_days: Option<i64>,
    ) -> Result<serde_json::Value> {
        tracing::info!(status = ?status, age_days = ?age_days, "Retrieving jobs");
        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => {
                let jobs = pg::get_jobs(pool, status, age_days)
                    .await
                    .map_err(Box::from)?;
                Ok(serde_json::to_value(jobs)?)
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => {
                let jobs = sqlt::get_jobs(pool, status, age_days)
                    .await
                    .map_err(Box::from)?;

                Ok(serde_json::to_value(jobs)?)
            }
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _, _) => {
                let jobs = redis::get_jobs(pool, status, age_days).await?;
                Ok(serde_json::to_value(jobs)?)
            }
            Self::None => {
                tracing::error!(
                    "No queue provider is configured: compile with at least one queue provider feature"
                );
                Err(Error::string("provider not configured"))
            }
        }
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

        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => pg::cancel_jobs_by_name(pool, job_name).await,
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => sqlt::cancel_jobs_by_name(pool, job_name).await,
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _, _) => redis::cancel_jobs_by_name(pool, job_name).await,
            Self::None => {
                tracing::error!(
                    "No queue provider is configured: compile with at least one queue provider feature"
                );
                Err(Error::string("provider not configured"))
            }
        }
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

        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => {
                pg::clear_jobs_older_than(pool, age_days, Some(status)).await
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => {
                sqlt::clear_jobs_older_than(pool, age_days, Some(status)).await
            }
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _, _) => {
                redis::clear_jobs_older_than(pool, age_days, Some(status)).await
            }
            Self::None => {
                tracing::error!(
                    "No queue provider is configured: compile with at least one queue provider feature"
                );
                Err(Error::string("provider not configured"))
            }
        }
    }

    /// Clears jobs based on their status for the configured queue provider.
    ///
    /// # Errors
    /// - If no queue provider is configured, it will return an error indicating the lack of configuration.
    /// - If the Redis provider is selected, it will return an error stating that clearing jobs is not supported.
    /// - Any error in the underlying provider's job clearing logic will propagate from the respective function.
    pub async fn clear_by_status(&self, status: Vec<JobStatus>) -> Result<()> {
        tracing::info!(status = ?status, "Clearing jobs by status");
        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => pg::clear_by_status(pool, status).await,
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => sqlt::clear_by_status(pool, status).await,
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _, _) => redis::clear_by_status(pool, status).await,
            Self::None => {
                tracing::error!(
                    "No queue provider is configured: compile with at least one queue provider feature"
                );
                Err(Error::string("provider not configured"))
            }
        }
    }

    /// Requeued job with the given minutes ages.
    ///
    /// # Errors
    /// - If no queue provider is configured, it will return an error indicating the lack of configuration.
    /// - If the Redis provider is selected, it will return an error stating that clearing jobs is not supported.
    /// - Any error in the underlying provider's job clearing logic will propagate from the respective function.
    pub async fn requeue(&self, age_minutes: &i64) -> Result<()> {
        tracing::info!(age_minutes = age_minutes, "Requeuing stale jobs");
        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _, _) => pg::requeue(pool, age_minutes).await,
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _, _) => sqlt::requeue(pool, age_minutes).await,
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _, _) => redis::requeue(pool, age_minutes).await,
            Self::None => {
                tracing::error!(
                    "No queue provider is configured: compile with at least one queue provider feature"
                );
                Err(Error::string("provider not configured"))
            }
        }
    }

    /// Dumps the list of jobs to a YAML file at the specified path.
    ///
    /// This function retrieves jobs from the queue, optionally filtered by their status, and
    /// writes the job data to a YAML file.
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

        let data = serde_yaml::to_string(&jobs)?;
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
    /// - If the queue provider is Redis or none, an error will be returned indicating the lack of support.
    /// - If any issues occur while enqueuing the jobs, the function will return an error.
    ///
    pub async fn import(&self, path: &Path) -> Result<()> {
        tracing::info!(path = %path.display(), "Importing jobs from file");

        match &self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, _, _, _) => {
                let jobs: Vec<pg::Job> = serde_yaml::from_reader(File::open(path)?)?;
                for job in jobs {
                    self.enqueue(job.name.clone(), None, job.data, None).await?;
                }

                Ok(())
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(_, _, _, _) => {
                let jobs: Vec<sqlt::Job> = serde_yaml::from_reader(File::open(path)?)?;
                for job in jobs {
                    self.enqueue(job.name.clone(), None, job.data, None).await?;
                }
                Ok(())
            }
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _, _) => {
                let jobs: Vec<redis::Job> = serde_yaml::from_reader(File::open(path)?)?;
                for job in jobs {
                    self.enqueue(job.name.clone(), None, job.data, None).await?;
                }
                Ok(())
            }
            Self::None => {
                tracing::error!(
                    "No queue provider is configured: compile with at least one queue provider feature"
                );
                Err(Error::string("provider not configured"))
            }
        }
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
    async fn perform_later(ctx: &AppContext, args: A) -> crate::Result<()>
    where
        Self: Sized,
    {
        match &ctx.config.workers.mode {
            WorkerMode::BackgroundQueue => {
                if let Some(p) = &ctx.queue_provider {
                    let tags = Self::tags();
                    let tags_option = if tags.is_empty() { None } else { Some(tags) };
                    p.enqueue(Self::class_name(), Self::queue(), args, tags_option)
                        .await?;
                } else {
                    tracing::error!(
                        "perform_later: background queue is selected, but queue was not populated \
                         in context"
                    );
                }
            }
            WorkerMode::ForegroundBlocking => {
                Self::build(ctx).perform(args).await?;
            }
            WorkerMode::BackgroundAsync => {
                let dx = ctx.clone();
                tokio::spawn(async move {
                    if let Err(err) = Self::build(&dx).perform(args).await {
                        tracing::error!(err = err.to_string(), "worker failed to perform job");
                    }
                });
            }
        }
        Ok(())
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
        })
        | QueueConfig::Redis(RedisQueueConfig {
            dangerously_flush,
            uri: _,
            queues: _,
            num_workers: _,
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
                #[cfg(feature = "bg_redis")]
                config::QueueConfig::Redis(qcfg) => {
                    tracing::debug!("Creating Redis queue provider");
                    Ok(Some(Arc::new(redis::create_provider(qcfg).await?)))
                }
                #[cfg(feature = "bg_pg")]
                config::QueueConfig::Postgres(qcfg) => {
                    tracing::debug!("Creating Postgres queue provider");
                    Ok(Some(Arc::new(pg::create_provider(qcfg).await?)))
                }
                #[cfg(feature = "bg_sqlt")]
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
