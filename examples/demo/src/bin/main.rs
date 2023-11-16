use blo::app::App;
use migration::Migrator;
use rustyrails::cli;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    cli::main::<App, Migrator>().await
}
