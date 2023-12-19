use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks},
    controller::AppRoutes,
    task::Tasks,
    worker::Processor,
};

use crate::controllers;

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn routes() -> AppRoutes {
        AppRoutes::empty()
            .prefix("/api")
            .add_route(controllers::home::routes())
    }

    fn connect_workers<'a>(_p: &'a mut Processor, _ctx: &'a AppContext) {}

    fn register_tasks(_tasks: &mut Tasks) {}
}
