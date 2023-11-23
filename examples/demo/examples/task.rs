use std::{collections::BTreeMap, env};

use blo::app::App;
use loco_rs::boot::{create_context, run_task};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let environment = std::env::var("RR_ENV")
        .or_else(|_| env::var("RAILS_ENV"))
        .or_else(|_| env::var("NODE_ENV"))
        .unwrap_or_else(|_| "development".to_string());

    let args = env::args().collect::<Vec<_>>();
    let cmd = args.get(1);
    let app_context = create_context(&environment).await?;
    run_task::<App>(&app_context, cmd, &BTreeMap::new()).await?;

    Ok(())
}
