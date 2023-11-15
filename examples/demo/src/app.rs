use std::path::Path;

use async_trait::async_trait;
use framework::{
    app::{AppContext, Hooks},
    controller::AppRoutes,
    db::{self, truncate_table},
    task::Tasks,
    worker::{AppWorker, Processor},
    Result,
};
use sea_orm::DatabaseConnection;

use crate::{
    controllers,
    models::_entities::{notes, users},
    tasks::user_report::UserReport,
    workers::downloader::DownloadWorker,
};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::notes::routes())
            .add_route(controllers::auth::routes())
            .add_route(controllers::user::routes())
    }

    fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {
        p.register(DownloadWorker::build(ctx));
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(UserReport);
    }

    async fn truncate(db: &DatabaseConnection) -> Result<()> {
        truncate_table(db, users::Entity).await?;
        truncate_table(db, notes::Entity).await?;
        Ok(())
    }

    async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(db, &base.join("users.yaml").display().to_string()).await?;
        Ok(())
    }
}
