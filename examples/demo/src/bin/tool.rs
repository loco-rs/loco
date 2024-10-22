use demo_app::app::App;
use loco_rs::{cli, tokio};
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
