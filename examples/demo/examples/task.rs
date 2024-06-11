use std::env;

use blo::app::App;
use loco_rs::{
    boot::{create_context, run_task},
    environment::{resolve_from_env, Environment},
    task,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let environment: Environment = resolve_from_env().into();

    let args = env::args().collect::<Vec<_>>();
    let cmd = args.get(1);
    let app_context = create_context::<App>(&environment).await?;
    run_task::<App>(&app_context, cmd, &task::Vars::default()).await?;

    Ok(())
}
