use blo::app::App;
use loco_rs::{
    boot::{create_app, start, StartMode},
    environment::{resolve_from_env, Environment},
};
use migration::Migrator;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let environment: Environment = resolve_from_env().into();

    let boot_result = create_app::<App, Migrator>(StartMode::ServerAndWorker, &environment).await?;
    start(boot_result).await?;
    Ok(())
}
