use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks},
    controller::AppRoutes,
    task::Tasks,
    worker::{AppWorker, Processor},
};

use crate::{controllers, tasks, workers::downloader::DownloadWorker};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes().add_route(controllers::foo::routes())
    }

    fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {
        p.register(DownloadWorker::build(ctx));
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::example::ExpReport);
    }
}
