use std::path::Path;

use async_trait::async_trait;
use loco_extras;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    boot::{create_app, BootResult, StartMode},
    cache,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    prelude::*,
    storage::{self, Storage},
    task::Tasks,
    Result,
};
use migration::Migrator;
use sea_orm::DatabaseConnection;

use crate::{
    controllers::{self, middlewares},
    initializers,
    models::_entities::{notes, roles, users, users_roles},
    tasks,
    workers::downloader::DownloadWorker,
};

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

    // <snip id="app-initializers">
    async fn initializers(ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        let mut initializers: Vec<Box<dyn Initializer>> = vec![
            Box::new(initializers::axum_session::AxumSessionInitializer),
            Box::new(initializers::view_engine::ViewEngineInitializer),
            Box::new(initializers::hello_view_engine::HelloViewEngineInitializer),
            Box::new(loco_extras::initializers::normalize_path::NormalizePathInitializer),
        ];

        if ctx.environment != Environment::Test {
            initializers.push(Box::new(
                loco_extras::initializers::prometheus::AxumPrometheusInitializer,
            ));
        }

        Ok(initializers)
    }
    // </snip>

    fn routes(ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(
                controllers::mylayer::routes(ctx.clone())
                    .layer(middlewares::routes::role::RoleRouteLayer::new(ctx.clone())),
            )
            .add_route(controllers::notes::routes())
            .add_route(controllers::auth::routes())
            .add_route(controllers::mysession::routes())
            .add_route(controllers::view_engine::routes())
            .add_route(controllers::user::routes())
            .add_route(controllers::upload::routes())
            .add_route(controllers::responses::routes())
            .add_route(controllers::cache::routes())
    }

    async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment).await
    }

    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        let store = if ctx.environment == Environment::Test {
            storage::drivers::mem::new()
        } else {
            storage::drivers::local::new_with_prefix("storage-uploads").map_err(Box::from)?
        };

        Ok(AppContext {
            storage: Storage::single(store).into(),
            cache: cache::Cache::new(cache::drivers::inmem::new()).into(),
            ..ctx
        })

        // Ok(ctx)
    }

    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::user_report::UserReport);
        tasks.register(tasks::seed::SeedData);
        tasks.register(tasks::foo::Foo);
    }

    async fn truncate(db: &DatabaseConnection) -> Result<()> {
        truncate_table(db, users_roles::Entity).await?;
        truncate_table(db, roles::Entity).await?;
        truncate_table(db, users::Entity).await?;
        truncate_table(db, notes::Entity).await?;
        Ok(())
    }

    async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(db, &base.join("users.yaml").display().to_string()).await?;
        db::seed::<notes::ActiveModel>(db, &base.join("notes.yaml").display().to_string()).await?;
        Ok(())
    }
}
