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
pub mod skq;
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
        bb8::Pool<sidekiq::RedisConnectionManager>,
        Arc<tokio::sync::Mutex<sidekiq::Processor>>,
        tokio_util::sync::CancellationToken,
    ),
    #[cfg(feature = "bg_pg")]
    Postgres(
        pg::PgPool,
        std::sync::Arc<tokio::sync::Mutex<pg::JobRegistry>>,
        pg::RunOpts,
    ),
    #[cfg(feature = "bg_sqlt")]
    Sqlite(
        sqlt::SqlitePool,
        std::sync::Arc<tokio::sync::Mutex<sqlt::JobRegistry>>,
        sqlt::RunOpts,
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
    ) -> Result<()> {
        tracing::debug!(worker = class, "job enqueue");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _) => {
                skq::enqueue(pool, class, queue, args).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                pg::enqueue(
                    pool,
                    &class,
                    serde_json::to_value(args)?,
                    chrono::Utc::now(),
                    None,
                )
                .await
                .map_err(Box::from)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => {
                sqlt::enqueue(
                    pool,
                    &class,
                    serde_json::to_value(args)?,
                    chrono::Utc::now(),
                    None,
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
        tracing::debug!(worker = W::class_name(), "register worker");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, p, _) => {
                let mut p = p.lock().await;
                p.register(skq::SidekiqBackgroundWorker::new(worker));
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, registry, _) => {
                let mut r = registry.lock().await;
                r.register_worker(W::class_name(), worker)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(_, registry, _) => {
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
    pub async fn run(&self) -> Result<()> {
        tracing::debug!("running background jobs");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, p, _) => {
                p.lock().await.clone().run().await;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, registry, run_opts) => {
                //TODOQ: num workers to config
                let handles = registry.lock().await.run(pool, run_opts);
                for handle in handles {
                    handle.await?;
                }
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, registry, run_opts) => {
                //TODOQ: num workers to config
                let handles = registry.lock().await.run(pool, run_opts);
                for handle in handles {
                    handle.await?;
                }
            }
            _ => {
                tracing::error!(
                    "no queue provider is configured: compile with at least one queue provider \
                     feature"
                );
            }
        }
        Ok(())
    }

    /// Runs the setup of this [`Queue`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn setup(&self) -> Result<()> {
        tracing::debug!("workers setup");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _) => {}
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                pg::initialize_database(pool).await.map_err(Box::from)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => {
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
        tracing::debug!("clearing job");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _) => {
                skq::clear(pool).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                pg::clear(pool).await.map_err(Box::from)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => {
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
        tracing::debug!("job queue ping requested");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _, _) => {
                skq::ping(pool).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                pg::ping(pool).await.map_err(Box::from)?;
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => {
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
            Self::Redis(_, _, _) => "redis queue".to_string(),
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, _, _) => "postgres queue".to_string(),
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(_, _, _) => "sqlite queue".to_string(),
            _ => "no queue".to_string(),
        }
    }

    /// # Errors
    ///
    /// Does not currently return an error, but the postgres or other future
    /// queue implementations might, so using Result here as return type.
    pub fn shutdown(&self) -> Result<()> {
        tracing::debug!("waiting for running jobs to finish...");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, cancellation_token) => cancellation_token.cancel(),
            _ => {}
        }

        Ok(())
    }

    async fn get_jobs(
        &self,
        status: Option<&Vec<JobStatus>>,
        age_days: Option<i64>,
    ) -> Result<serde_json::Value> {
        tracing::debug!(status = ?status, age_days = ?age_days, "getting jobs");
        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                let jobs = pg::get_jobs(pool, status, age_days)
                    .await
                    .map_err(Box::from)?;
                Ok(serde_json::to_value(jobs)?)
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => {
                let jobs = sqlt::get_jobs(pool, status, age_days)
                    .await
                    .map_err(Box::from)?;

                Ok(serde_json::to_value(jobs)?)
            }
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _) => {
                tracing::error!("getting jobs for redis provider not implemented");
                Err(Error::string(
                    "getting jobs not supported for redis provider",
                ))
            }
            Self::None => {
                tracing::error!(
                    "no queue provider is configured: compile with at least one queue provider \
                     feature"
                );
                Err(Error::string("provider not configure"))
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
        tracing::debug!(job_name = ?job_name, "cancel jobs");

        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => pg::cancel_jobs_by_name(pool, job_name).await,
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => sqlt::cancel_jobs_by_name(pool, job_name).await,
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _) => {
                tracing::error!("canceling jobs for redis provider not implemented");
                Err(Error::string(
                    "canceling jobs not supported for redis provider",
                ))
            }
            Self::None => {
                tracing::error!(
                    "no queue provider is configured: compile with at least one queue provider \
                     feature"
                );
                Err(Error::string("provider not configure"))
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
        tracing::debug!(age_days = age_days, status = ?status, "cancel jobs with age");

        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                pg::clear_jobs_older_than(pool, age_days, Some(status)).await
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => {
                sqlt::clear_jobs_older_than(pool, age_days, Some(status)).await
            }
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _) => {
                tracing::error!("clear jobs for redis provider not implemented");
                Err(Error::string("clear jobs not supported for redis provider"))
            }
            Self::None => {
                tracing::error!(
                    "no queue provider is configured: compile with at least one queue provider \
                     feature"
                );
                Err(Error::string("provider not configure"))
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
        tracing::debug!(status = ?status, "clear jobs by status");
        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => pg::clear_by_status(pool, status).await,
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => sqlt::clear_by_status(pool, status).await,
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _) => {
                tracing::error!("clear jobs for redis provider not implemented");
                Err(Error::string("clear jobs not supported for redis provider"))
            }
            Self::None => {
                tracing::error!(
                    "no queue provider is configured: compile with at least one queue provider \
                     feature"
                );
                Err(Error::string("provider not configure"))
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
        tracing::debug!(age_minutes = age_minutes, "requeue jobs");
        match self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => pg::requeue(pool, age_minutes).await,
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(pool, _, _) => sqlt::requeue(pool, age_minutes).await,
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _) => {
                tracing::error!("Update status for redis provider not implemented");
                Err(Error::string(
                    "Update status not supported for redis provider",
                ))
            }
            Self::None => {
                tracing::error!(
                    "no queue provider is configured: compile with at least one queue provider \
                     feature"
                );
                Err(Error::string("provider not configure"))
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
        tracing::debug!(path = %path.display(), status = ?status, age_days = ?age_days, "dumping jobs");

        if !path.exists() {
            tracing::debug!(path = %path.display(), "folder not exists, creating...");
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
        tracing::debug!(path = %path.display(), "import jobs");

        match &self {
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, _, _) => {
                let jobs: Vec<pg::Job> = serde_yaml::from_reader(File::open(path)?)?;
                for job in jobs {
                    self.enqueue(job.name.to_string(), None, job.data).await?;
                }

                Ok(())
            }
            #[cfg(feature = "bg_sqlt")]
            Self::Sqlite(_, _, _) => {
                let jobs: Vec<sqlt::Job> = serde_yaml::from_reader(File::open(path)?)?;
                for job in jobs {
                    self.enqueue(job.name.to_string(), None, job.data).await?;
                }
                Ok(())
            }
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _, _) => {
                tracing::error!("import jobs for redis provider not implemented");
                Err(Error::string(
                    "getting jobs not supported for redis provider",
                ))
            }
            Self::None => {
                tracing::error!(
                    "no queue provider is configured: compile with at least one queue provider \
                     feature"
                );
                Err(Error::string("provider not configure"))
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
                    p.enqueue(Self::class_name(), Self::queue(), args).await?;
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
                // TODOQ call the object inside RedisQueueConfig and pass that
                #[cfg(feature = "bg_redis")]
                config::QueueConfig::Redis(qcfg) => {
                    Ok(Some(Arc::new(skq::create_provider(qcfg).await?)))
                }
                #[cfg(feature = "bg_pg")]
                config::QueueConfig::Postgres(qcfg) => {
                    Ok(Some(Arc::new(pg::create_provider(qcfg).await?)))
                }
                #[cfg(feature = "bg_sqlt")]
                config::QueueConfig::Sqlite(qcfg) => {
                    Ok(Some(Arc::new(sqlt::create_provider(qcfg).await?)))
                }

                #[allow(unreachable_patterns)]
                _ => Err(Error::string(
                    "no queue provider feature was selected and compiled, but queue configuration \
                     is present",
                )),
            }
        } else {
            Ok(None)
        }
    } else {
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

        assert_debug_snapshot!(std::fs::read_to_string(dump_file));
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
