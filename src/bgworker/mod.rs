use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use tracing::{debug, error};
#[cfg(feature = "bg_pg")]
pub mod pg;
#[cfg(feature = "bg_redis")]
pub mod skq;

use crate::{
    app::AppContext,
    config::{self, Config, PostgresQueueConfig, QueueConfig, RedisQueueConfig, WorkerMode},
    Error, Result,
};

// Queue struct now holds both a QueueProvider and QueueRegistrar
pub enum Queue {
    #[cfg(feature = "bg_redis")]
    Redis(
        bb8::Pool<sidekiq::RedisConnectionManager>,
        Arc<tokio::sync::Mutex<sidekiq::Processor>>,
    ),
    #[cfg(feature = "bg_pg")]
    Postgres(
        pg::PgPool,
        std::sync::Arc<tokio::sync::Mutex<pg::TaskRegistry>>,
        pg::RunOpts,
    ),
    None,
}

impl Queue {
    /// Add a job to the queue
    ///
    /// # Errors
    ///
    /// This function will return an error if fails
    pub async fn enqueue<A: Serialize + Send + Sync>(
        &self,
        class: String,
        queue: Option<String>,
        args: A,
    ) -> Result<()> {
        debug!(worker = class, "job enqueue");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _) => {
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
            _ => {}
        }
        Ok(())
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
        debug!(worker = W::class_name(), "register worker");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, p) => {
                let mut p = p.lock().await;
                p.register(skq::SidekiqBackgroundWorker::new(worker));
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, registry, _) => {
                let mut r = registry.lock().await;
                r.register_worker(W::class_name(), worker);
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
        debug!("running background jobs");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, p) => {
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
            _ => {
                error!(
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
        debug!("workers setup");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _) => {}
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                pg::initialize_database(pool).await.map_err(Box::from)?;
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
        debug!("clearing job queues");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _) => {
                skq::clear(pool).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                pg::clear(pool).await.map_err(Box::from)?;
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
        debug!("job queue ping requested");
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(pool, _) => {
                skq::ping(pool).await?;
            }
            #[cfg(feature = "bg_pg")]
            Self::Postgres(pool, _, _) => {
                pg::ping(pool).await.map_err(Box::from)?;
            }
            _ => {}
        }
        Ok(())
    }

    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            #[cfg(feature = "bg_redis")]
            Self::Redis(_, _) => "redis queue".to_string(),
            #[cfg(feature = "bg_pg")]
            Self::Postgres(_, _, _) => "postgres queue".to_string(),
            _ => "no queue".to_string(),
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
                    error!(
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
                        error!(err = err.to_string(), "worker failed to perform job");
                    }
                });
            }
        }
        Ok(())
    }

    async fn perform(&self, args: A) -> crate::Result<()>;
}

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
