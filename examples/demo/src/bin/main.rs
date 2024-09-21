use demo_app::app::App;
use loco_rs::{app::AppContext, cli};
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<AppContext, App, Migrator>().await
}
