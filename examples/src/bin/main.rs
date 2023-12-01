use blo::app::App;
use loco_rs::cli;
use migration::Migrator;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    cli::main::<App, Migrator>().await
}
