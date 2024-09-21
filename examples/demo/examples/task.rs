use std::env;

use demo_app::app::App;
use loco_rs::{
    app::AppContext,
    boot::{create_context, run_task},
    environment::{resolve_from_env, Environment},
    task,
};

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    let environment: Environment = resolve_from_env().into();

    let args = env::args().collect::<Vec<_>>();
    let cmd = args.get(1);
    let app_context = create_context::<AppContext, App>(&environment).await?;
    run_task::<AppContext, App>(&app_context, cmd, &task::Vars::default()).await?;

    Ok(())
}
