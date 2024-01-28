use std::{env, str::FromStr};

use blo::app::App;
use loco_rs::{
    boot::{create_app, start, ServeConfig, StartMode},
    environment::{resolve_from_env, Environment},
};
use migration::Migrator;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let environment: Environment = resolve_from_env().into();

    let boot_result = create_app::<App, Migrator>(StartMode::WorkerOnly, &environment).await?;
    let serve_config = ServeConfig {
        port: boot_result.app_context.config.server.port,
        binding: boot_result.app_context.config.server.binding.to_string(),
    };
    start(boot_result, serve_config).await?;
    Ok(())
}
