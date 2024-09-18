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

pub type AppWorkerOpts<Args, T> = sidekiq::WorkerOpts<Args, T>;

#[async_trait]
#[allow(clippy::module_name_repetitions)]
pub trait AppWorker<T>: Worker<T>
where
    Self: Sized,
    T: Send + Sync + serde::Serialize + 'static,
{
    fn build(ctx: &AppContext) -> Self;

    async fn perform_in(ctx: &AppContext, duration: std::time::Duration, args: T) -> Result<()> {
        match &ctx.config.workers.mode {
            WorkerMode::BackgroundQueue => {
                if let Some(queue) = &ctx.queue {
                    Self::opts()
                        .perform_in(queue, duration, args)
                        .await
                        .unwrap();
                } else {
                    error!(
                        error.msg =
                            "worker mode requested but no queue connection supplied, skipping job",
                        "worker_error"
                    );
                }
            }
            WorkerMode::ForegroundBlocking => {
                std::thread::sleep(duration);
                Self::build(ctx).perform(args).await.unwrap();
            }
            WorkerMode::BackgroundAsync => {
                let dx = ctx.clone();
                tokio::spawn(async move {
                    // If wait time is larger than a minute (+ 4 seconds), wait in
                    // intervals of 1 minute to avoid long running tasks in the
                    // event loop, as well as computer sleep mode or clock changing.
                    if duration > std::time::Duration::from_secs(64) {
                        // Give 4 seconds buffer for waking up
                        let num_sleeps = duration.as_secs() / 60;
                        for _ in 0..num_sleeps {
                            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                        }
                    } else {
                        tokio::time::sleep(duration).await;
                    }

                    Self::build(&dx).perform(args).await
                });
            }
        }
        Ok(())
    }

    async fn perform_later(ctx: &AppContext, args: T) -> Result<()> {
        match &ctx.config.workers.mode {
            WorkerMode::BackgroundQueue => {
                if let Some(queue) = &ctx.queue {
                    Self::perform_async(queue, args).await.unwrap();
                } else {
                    error!(
                        error.msg =
                            "worker mode requested but no queue connection supplied, skipping job",
                        "worker_error"
                    );
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
