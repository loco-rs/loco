use loco_rs::cli;
use migration::Migrator;
use reference_spa::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
