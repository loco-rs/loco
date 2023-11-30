use std::env;

use blo::app::App;
use loco_rs::boot::{create_app, start, StartMode};
use migration::Migrator;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let environment = std::env::var("RR_ENV")
        .or_else(|_| env::var("RAILS_ENV"))
        .or_else(|_| env::var("NODE_ENV"))
        .unwrap_or_else(|_| "development".to_string());

    let boot_result = create_app::<App, Migrator>(StartMode::ServerAndWorker, &environment).await?;
    start(boot_result).await?;
    Ok(())
}
