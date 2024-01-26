use std::{collections::BTreeMap, env};

use blo::app::App;
use loco_rs::{
    boot::{create_context, run_task},
    config::ConfigOverrides,
    environment::{resolve_from_env, Environment},
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let environment: Environment = resolve_from_env().into();
    let config_overrides = ConfigOverrides::default();

    let args = env::args().collect::<Vec<_>>();
    let cmd = args.get(1);
    let app_context = create_context::<App>(&environment, &config_overrides).await?;
    run_task::<App>(&app_context, cmd, &BTreeMap::new()).await?;

    Ok(())
}
