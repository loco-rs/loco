use async_trait::async_trait;
pub use bb8::Pool;
pub use sidekiq::{Processor, RedisConnectionManager, Result, Worker};
use tracing::error;

use super::{app::AppContext, config::WorkerMode};
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

#[async_trait]
#[allow(clippy::module_name_repetitions)]
pub trait AppWorker<T>: Worker<T>
where
    Self: Sized,
    T: Send + Sync + serde::Serialize + 'static,
{
    fn build(ctx: &AppContext) -> Self;
    async fn perform_later(ctx: &AppContext, args: T) -> Result<()> {
        match &ctx.config.workers.mode {
            WorkerMode::BackgroundQueue => {
                if let Some(redis) = &ctx.redis {
                    Self::perform_async(redis, args).await.unwrap();
                } else {
                    error!("worker mode requested but no redis connection supplied, skipping job");
                }
            }
            WorkerMode::ForegroundBlocking => {
                Self::build(ctx).perform(args).await.unwrap();
            }
            WorkerMode::BackgroundAsync => {
                let dx = ctx.clone();
                tokio::spawn(async move { Self::build(&dx).perform(args).await });
            }
        }
        Ok(())
    }
}
