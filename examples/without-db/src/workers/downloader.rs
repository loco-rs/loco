use async_trait::async_trait;
use rustyrails::{
    app::AppContext,
    worker::{AppWorker, Result, Worker},
};
use serde::{Deserialize, Serialize};

pub struct DownloadWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DownloadWorkerArgs {}

impl AppWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
}

#[async_trait]
impl Worker<DownloadWorkerArgs> for DownloadWorker {
    async fn perform(&self, _args: DownloadWorkerArgs) -> Result<()> {
        println!("Download worker started");
        Ok(())
    }
}
