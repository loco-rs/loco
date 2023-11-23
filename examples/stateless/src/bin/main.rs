use blo::app::App;
use rustyrails::cli;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    cli::main::<App>().await
}
