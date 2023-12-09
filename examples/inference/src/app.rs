use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks},
    controller::AppRoutes,
    task::Tasks,
    worker::Processor,
    Result,
};
use tracing::info;

use crate::{controllers, llm, tasks};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }
    async fn before_run(_ctx: &AppContext) -> Result<()> {
        // force static load now
        info!("before_run: loading model...");
        llm::model::load();
        info!("before_run: loading model done");
        Ok(())
    }

    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes().add_route(controllers::home::routes())
    }

    fn connect_workers<'a>(_p: &'a mut Processor, _ctx: &'a AppContext) {}

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::example::ExpReport);
    }
}
