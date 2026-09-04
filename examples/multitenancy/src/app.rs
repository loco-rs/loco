use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

use crate::{
    controllers,
    models::_entities::{
        applications, clients, documents, invoices, permissions, projects, role_permissions, roles,
        tenant_applications, tenant_member_roles, tenant_members, tenants, users,
    },
    tasks,
    workers::downloader::DownloadWorker,
};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::clients::routes())
            .add_route(controllers::projects::routes())
            .add_route(controllers::documents::routes())
            .add_route(controllers::invoices::routes())
            .add_route(controllers::purchases::routes())
            .add_route(controllers::dashboard::routes())
            .add_route(controllers::auth::routes())
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
        tasks.register(tasks::user_create::UserCreate);
        tasks.register(tasks::user_delete::UserDelete);
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, role_permissions::Entity).await?;
        truncate_table(&ctx.db, tenant_member_roles::Entity).await?;
        truncate_table(&ctx.db, permissions::Entity).await?;
        truncate_table(&ctx.db, clients::Entity).await?;
        truncate_table(&ctx.db, projects::Entity).await?;
        truncate_table(&ctx.db, documents::Entity).await?;
        truncate_table(&ctx.db, invoices::Entity).await?;
        truncate_table(&ctx.db, roles::Entity).await?;
        truncate_table(&ctx.db, tenant_members::Entity).await?;
        truncate_table(&ctx.db, tenant_applications::Entity).await?;
        truncate_table(&ctx.db, applications::Entity).await?;
        truncate_table(&ctx.db, tenants::Entity).await?;
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<applications::ActiveModel>(
            &ctx.db,
            &base.join("applications.yaml").display().to_string(),
        )
        .await?;
        Ok(())
    }
}
