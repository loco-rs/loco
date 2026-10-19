use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::users;

/// Deletes users who never verified their address.
///
/// This is a `BackgroundWorker`, not a `tokio::spawn`: the request that
/// triggers it must return immediately, and the work has to survive a restart.
/// A spawned future is neither durable, retried, nor observable.
pub struct PurgeUnverifiedWorker {
    pub ctx: AppContext,
}

/// Args are persisted to the queue, so they carry values — never a `Model`,
/// which would be a stale snapshot by the time the job runs.
#[derive(Deserialize, Debug, Serialize)]
pub struct PurgeUnverifiedWorkerArgs {
    pub older_than_days: i64,
}

#[async_trait]
impl BackgroundWorker<PurgeUnverifiedWorkerArgs> for PurgeUnverifiedWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: PurgeUnverifiedWorkerArgs) -> Result<()> {
        // Failure returns Err and the queue decides whether to retry. A
        // hand-rolled sleep-and-count loop would block a worker slot and lose
        // its state on restart.
        let removed =
            users::Model::purge_unverified_older_than(&self.ctx.db, args.older_than_days).await?;

        tracing::info!(removed, days = args.older_than_days, "purged unverified users");
        Ok(())
    }
}
