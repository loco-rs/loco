use std::time::Duration;

use async_trait::async_trait;
use loco_rs::{
    app::AppContext,
    worker::{AppWorker, Result, Worker},
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::models::users;

pub struct DownloadWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DownloadWorkerArgs {
    pub user_guid: String,
}

impl AppWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
}

#[async_trait]
impl Worker<DownloadWorkerArgs> for DownloadWorker {
    async fn perform(&self, args: DownloadWorkerArgs) -> Result<()> {
        // TODO: Some actual work goes here...
        println!("================================================");
        println!("Sending payment report to user {}", args.user_guid);

        sleep(Duration::from_millis(2000)).await;

        let all = users::Entity::find()
            .all(&self.ctx.db)
            .await
            .map_err(Box::from)?;
        for post in &all {
            let notes = post.notes(&self.ctx.db).await;
            println!("post: {} {:?}", post.id, notes);
        }
        println!("================================================");
        Ok(())
    }
}
