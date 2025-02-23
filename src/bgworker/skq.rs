use std::{marker::PhantomData, panic::AssertUnwindSafe, sync::Arc};

use async_trait::async_trait;
use bb8::Pool;
use futures_util::FutureExt;
use sidekiq::{Processor, ProcessorConfig, RedisConnectionManager};

use super::{BackgroundWorker, Queue};
use crate::{config::RedisQueueConfig, Result};
pub type RedisPool = Pool<RedisConnectionManager>;

#[derive(Debug)]
pub struct SidekiqBackgroundWorker<W, A> {
    pub inner: W, // Now we store the worker with its actual type instead of a trait object
    _phantom: PhantomData<A>,
}
impl<W, A> SidekiqBackgroundWorker<W, A>
where
    W: BackgroundWorker<A> + 'static,
    A: Send + Sync + serde::Serialize + 'static,
{
    pub fn new(worker: W) -> Self {
        Self {
            inner: worker,
            _phantom: PhantomData, // Initialize PhantomData for A
        }
    }
}

#[async_trait]
impl<W, A> sidekiq::Worker<A> for SidekiqBackgroundWorker<W, A>
where
    W: BackgroundWorker<A> + Send + Sync + 'static,
    A: Send + Sync + serde::Serialize + 'static,
{
    fn class_name() -> String
    where
        Self: Sized,
    {
        // Now we can use the worker's static class_name method
        W::class_name()
    }

    async fn perform(&self, args: A) -> sidekiq::Result<()> {
        // Forward the perform call to the inner worker
        match AssertUnwindSafe(self.inner.perform(args))
            .catch_unwind()
            .await
        {
            Ok(result) => result.map_err(|e| sidekiq::Error::Any(Box::from(e))),
            Err(panic) => {
                let panic_msg = panic
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
                    .or_else(|| panic.downcast_ref::<&str>().copied())
                    .unwrap_or("Unknown panic occurred");
                tracing::error!(err = panic_msg, "worker panicked");
                Err(sidekiq::Error::Any(Box::from(panic_msg)))
            }
        }
    }
}
/// Clear tasks
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn clear(pool: &RedisPool) -> Result<()> {
    let mut conn = pool.get().await?;
    sidekiq::redis_rs::cmd("FLUSHDB")
        .query_async::<_, ()>(conn.unnamespaced_borrow_mut())
        .await?;
    Ok(())
}

/// Add a task
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn enqueue(
    pool: &RedisPool,
    class: String,
    queue: Option<String>,
    args: impl serde::Serialize + Send,
) -> Result<()> {
    sidekiq::opts()
        .queue(queue.unwrap_or_else(|| "default".to_string()))
        .perform_async(pool, class, args)
        .await
        .map_err(Box::from)?;
    Ok(())
}

/// Ping system
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn ping(pool: &RedisPool) -> Result<()> {
    let mut conn = pool.get().await?;
    Ok(sidekiq::redis_rs::cmd("PING")
        .query_async::<_, ()>(conn.unnamespaced_borrow_mut())
        .await?)
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
/// Create this provider
///
/// # Errors
///
/// This function will return an error if it fails
pub async fn create_provider(qcfg: &RedisQueueConfig) -> Result<Queue> {
    let manager = RedisConnectionManager::new(qcfg.uri.clone())?;
    let redis = Pool::builder().build(manager).await?;
    let queues = get_queues(&qcfg.queues);
    let processor = Processor::new(redis.clone(), queues)
        .with_config(ProcessorConfig::default().num_workers(qcfg.num_workers as usize));
    let cancellation_token = processor.get_cancellation_token();

    Ok(Queue::Redis(
        redis,
        Arc::new(tokio::sync::Mutex::new(processor)),
        cancellation_token,
    ))
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::get_queues;

    #[test]
    fn test_default_custom_queues() {
        let default_queues = get_queues(&None);
        assert_debug_snapshot!(default_queues);

        let default_queues2 = get_queues(&Some(vec![]));
        assert_debug_snapshot!(default_queues2);

        let merged_queues = get_queues(&Some(vec!["foo".to_string(), "bar".to_string()]));
        assert_debug_snapshot!(merged_queues);
    }
}
