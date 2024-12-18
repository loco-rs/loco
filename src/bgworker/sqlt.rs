/// `SQLite` based background job queue provider
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use apalis::{
    layers::WorkerBuilderExt,
    prelude::{service_fn, Request, Storage, WorkerBuilder, WorkerFactory},
};
use apalis_sql::{context::SqlContext, sqlite::SqliteStorage, Config};
use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
pub use sqlx::SqlitePool;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    ConnectOptions,
};
use tokio::task::JoinHandle;
use tower::util::BoxCloneService;
use tracing::debug;

use super::{BackgroundWorker, Queue};
use crate::{config::SqliteQueueConfig, Result};
type TaskId = String;
type TaskData = JsonValue;
type TaskStatus = String;

type TaskHandler = BoxCloneService<Request<TaskData, SqlContext>, (), apalis::prelude::Error>;

#[derive(Debug, Deserialize, Serialize)]
struct Task {
    pub id: TaskId,
    pub name: String,
    #[allow(clippy::struct_field_names)]
    pub task_data: TaskData,
    pub status: TaskStatus,
    pub run_at: DateTime<Utc>,
    pub interval: Option<i64>,
}

pub struct TaskRegistry {
    handlers: HashMap<String, TaskHandler>,
}

impl TaskRegistry {
    /// Creates a new `TaskRegistry`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registers a task handler with the provided name.
    /// # Errors
    /// Fails if cannot register worker
    pub fn register_worker<Args, W>(&mut self, name: String, worker: W) -> Result<()>
    where
        Args: Send + Serialize + Sync + 'static,
        W: BackgroundWorker<Args> + 'static,
        for<'de> Args: Deserialize<'de>,
    {
        let worker = Arc::new(worker);
        let wrapped_handler = move |task_data: TaskData| {
            let w = worker.clone();

            Box::pin(async move {
                let args = serde_json::from_value::<Args>(task_data);
                match args {
                    Ok(args) => w.perform(args).await,
                    Err(err) => Err(err.into()),
                }
            }) as Pin<Box<dyn Future<Output = Result<(), crate::Error>> + Send>>
        };
        let _ = &mut self
            .handlers
            .insert(name, BoxCloneService::new(service_fn(wrapped_handler)));
        Ok(())
    }

    /// Returns a reference to the task handlers.
    #[must_use]
    pub fn handlers(&self) -> &HashMap<String, TaskHandler> {
        &self.handlers
    }

    /// Runs the task handlers with the provided number of workers.
    #[must_use]
    pub fn run(&self, pool: &SqlitePool, opts: &RunOpts) -> Vec<JoinHandle<()>> {
        let mut tasks = Vec::new();
        for (name, handler) in self.handlers.iter() {
            let config = Config::new(name)
                .set_poll_interval(Duration::from_secs(opts.poll_interval_sec.into()));
            let backend = SqliteStorage::new_with_config(pool.clone(), config);
            let worker = WorkerBuilder::new(name)
                .concurrency(opts.num_workers as usize)
                .backend(backend)
                .build(handler.clone());
            let task = tokio::spawn(worker.run());
            tasks.push(task);
        }
        tasks
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
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

/// Initialize task tables
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn initialize_database(pool: &SqlitePool) -> Result<()> {
    debug!("sqlite worker: initialize database");
    SqliteStorage::setup(pool).await?;
    Ok(())
}

/// Add a task
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn enqueue(
    pool: &SqlitePool,
    name: &str,
    task_data_json: TaskData,
    run_at: DateTime<Utc>,
) -> Result<TaskId> {
    let mut storage: SqliteStorage<TaskData> =
        SqliteStorage::new_with_config(pool.clone(), Config::default().set_namespace(name));
    let req = storage
        .schedule_request(Request::new(task_data_json), run_at.timestamp())
        .await?;
    Ok(req.task_id.to_string())
}

/// Clear all tasks
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE from Jobs").execute(pool).await?;
    Ok(())
}

/// Ping system
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn ping(pool: &SqlitePool) -> Result<()> {
    sqlx::query("SELECT id from Jobs LIMIT 1")
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug)]
pub struct RunOpts {
    pub num_workers: u32,
    pub poll_interval_sec: u32,
}

/// Create this provider
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn create_provider(qcfg: &SqliteQueueConfig) -> Result<Queue> {
    let pool = connect(qcfg).await.map_err(Box::from)?;
    let registry = TaskRegistry::new();
    Ok(Queue::Sqlite(
        pool,
        Arc::new(tokio::sync::Mutex::new(registry)),
        RunOpts {
            num_workers: qcfg.num_workers,
            poll_interval_sec: qcfg.poll_interval_sec,
        },
    ))
}
