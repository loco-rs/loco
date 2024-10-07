/// Postgres based background job queue provider
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
pub use sqlx::PgPool;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgRow},
    ConnectOptions, Row,
};
use tokio::{task::JoinHandle, time::sleep};
use tracing::{debug, error, trace};
use ulid::Ulid;

use super::{BackgroundWorker, Queue};
use crate::{config::PostgresQueueConfig, Result};
type TaskId = String;
type TaskData = JsonValue;
type TaskStatus = String;

type TaskHandler = Box<
    dyn Fn(
            TaskId,
            TaskData,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<(), crate::Error>> + Send>>
        + Send
        + Sync,
>;

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
    handlers: Arc<HashMap<String, TaskHandler>>,
}

impl TaskRegistry {
    /// Creates a new `TaskRegistry`.
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(HashMap::new()),
        }
    }

    /// Registers a task handler with the provided name.
    pub fn register_worker<Args, W>(&mut self, name: String, worker: W)
    where
        Args: Send + Serialize + Sync + 'static,
        W: BackgroundWorker<Args> + 'static,
        for<'de> Args: Deserialize<'de>,
    {
        let worker = Arc::new(worker);
        let wrapped_handler = move |_task_id: String, task_data: TaskData| {
            let w = worker.clone();

            Box::pin(async move {
                let args = serde_json::from_value::<Args>(task_data);
                match args {
                    Ok(args) => w.perform(args).await,
                    Err(err) => Err(err.into()),
                }
            }) as Pin<Box<dyn Future<Output = Result<(), crate::Error>> + Send>>
        };

        Arc::get_mut(&mut self.handlers)
            .unwrap()
            .insert(name, Box::new(wrapped_handler));
    }

    /// Returns a reference to the task handlers.
    pub fn handlers(&self) -> &Arc<HashMap<String, TaskHandler>> {
        &self.handlers
    }

    /// Runs the task handlers with the provided number of workers.
    pub fn run(&self, pool: &PgPool, opts: &RunOpts) -> Vec<JoinHandle<()>> {
        let mut tasks = Vec::new();

        let interval = opts.poll_interval_sec;
        for idx in 0..opts.num_workers {
            let handlers = self.handlers.clone();

            let pool = pool.clone();
            let task = tokio::spawn(async move {
                loop {
                    trace!(
                        pool_conns = pool.num_idle(),
                        worker_num = idx,
                        "pg workers stats"
                    );
                    let task_opt = match dequeue(&pool).await {
                        Ok(t) => t,
                        Err(err) => {
                            error!(err = err.to_string(), "cannot fetch from queue");
                            None
                        }
                    };

                    if let Some(task) = task_opt {
                        debug!(task_id = task.id, name = task.name, "working on task");
                        if let Some(handler) = handlers.get(&task.name) {
                            match handler(task.id.clone(), task.task_data.clone()).await {
                                Ok(()) => {
                                    if let Err(err) =
                                        complete_task(&pool, &task.id, task.interval).await
                                    {
                                        error!(
                                            err = err.to_string(),
                                            task = ?task,
                                            "cannot complete task"
                                        );
                                    }
                                }
                                Err(err) => {
                                    if let Err(err) = fail_task(&pool, &task.id, &err).await {
                                        error!(
                                            err = err.to_string(),
                                            task = ?task,
                                            "cannot fail task"
                                        );
                                    }
                                }
                            }
                        } else {
                            error!(task = task.name, "no handler found for task");
                        }
                    } else {
                        sleep(Duration::from_secs(interval.into())).await;
                    }
                }
            });

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

pub async fn initialize_database(pool: &PgPool) -> Result<()> {
    debug!("pg worker: initialize database");
    sqlx::raw_sql(
        r"
            CREATE TABLE IF NOT EXISTS pg_loco_queue (
                id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                task_data JSONB NOT NULL,
                status VARCHAR NOT NULL DEFAULT 'queued',
                run_at TIMESTAMPTZ NOT NULL,
                interval BIGINT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            ",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn enqueue(
    pool: &PgPool,
    name: &str,
    task_data: TaskData,
    run_at: DateTime<Utc>,
    interval: Option<Duration>,
) -> Result<TaskId> {
    let task_data_json = serde_json::to_value(task_data)?;

    #[allow(clippy::cast_possible_truncation)]
    let interval_ms: Option<i64> = interval.map(|i| i.as_millis() as i64);

    let id = Ulid::new().to_string();
    sqlx::query(
        "INSERT INTO pg_loco_queue (id, task_data, name, run_at, interval) VALUES ($1, $2, $3, \
         $4, $5)",
    )
    .bind(id.clone())
    .bind(task_data_json)
    .bind(name)
    .bind(run_at)
    .bind(interval_ms)
    .execute(pool)
    .await?;
    Ok(id)
}

async fn dequeue(client: &PgPool) -> Result<Option<Task>> {
    let mut tx = client.begin().await?;
    let row = sqlx::query(
        "SELECT id, name, task_data, status, run_at, interval FROM pg_loco_queue WHERE status = \
         'queued' AND run_at <= NOW() ORDER BY run_at LIMIT 1 FOR UPDATE SKIP LOCKED",
    )
    // avoid using FromRow because it requires the 'macros' feature, which nothing
    // in our dep tree uses, so it'll create smaller, faster builds if we do this manually
    .map(|row: PgRow| Task {
        id: row.get("id"),
        name: row.get("name"),
        task_data: row.get("task_data"),
        status: row.get("status"),
        run_at: row.get("run_at"),
        interval: row.get("interval"),
    })
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(task) = row {
        sqlx::query(
            "UPDATE pg_loco_queue SET status = 'processing', updated_at = NOW() WHERE id = $1",
        )
        .bind(&task.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(task))
    } else {
        Ok(None)
    }
}

async fn complete_task(pool: &PgPool, task_id: &TaskId, interval_ms: Option<i64>) -> Result<()> {
    if let Some(interval_ms) = interval_ms {
        let next_run_at = Utc::now() + chrono::Duration::milliseconds(interval_ms);
        sqlx::query(
            "UPDATE pg_loco_queue SET status = 'queued', updated_at = NOW(), run_at = $1 WHERE id \
             = $2",
        )
        .bind(next_run_at)
        .bind(task_id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE pg_loco_queue SET status = 'completed', updated_at = NOW() WHERE id = $1",
        )
        .bind(task_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn fail_task(pool: &PgPool, task_id: &TaskId, error: &crate::Error) -> Result<()> {
    let msg = error.to_string();
    error!(err = msg, "failed task");
    let error_json = serde_json::json!({ "error": msg });
    sqlx::query(
        "UPDATE pg_loco_queue SET status = 'failed', updated_at = NOW(), task_data = task_data || \
         $1::jsonb WHERE id = $2",
    )
    .bind(error_json)
    .bind(task_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear(pool: &PgPool) -> Result<()> {
    sqlx::query("DELETE from pg_loco_queue")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn ping(pool: &PgPool) -> Result<()> {
    sqlx::query("SELECT id from pg_loco_queue LIMIT 1")
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug)]
pub struct RunOpts {
    pub num_workers: u32,
    pub poll_interval_sec: u32,
}

pub async fn create_provider(qcfg: &PostgresQueueConfig) -> Result<Queue> {
    let pool = connect(qcfg).await.map_err(Box::from)?;
    let registry = TaskRegistry::new();
    Ok(Queue::Postgres(
        pool,
        Arc::new(tokio::sync::Mutex::new(registry)),
        RunOpts {
            num_workers: qcfg.num_workers,
            poll_interval_sec: qcfg.poll_interval_sec,
        },
    ))
}
