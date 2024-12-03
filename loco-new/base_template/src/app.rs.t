{%- if settings.db %}
use std::path::Path;
{%- endif %}
use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{
        {%- if settings.background %}
        BackgroundWorker,
        {%- endif %}
        Queue},
    boot::{create_app, BootResult, StartMode},
    controller::AppRoutes,
    {%- if settings.db %}
    db::{self, truncate_table},
    {%- endif %}
    environment::Environment,
    task::Tasks,
    Result,
};
{%- if settings.db %}
use migration::Migrator;
use sea_orm::DatabaseConnection;
{%- endif %}

#[allow(unused_imports)]
use crate::{
    controllers
    {%- if settings.initializers -%}
    , initializers
    {%- endif %} 
    {%- if settings.db %}
    ,tasks
    , models::_entities::users
    {%- endif %}
    {%- if settings.background %}
    , workers::downloader::DownloadWorker
    {%- endif %},
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

    async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult> {
        {%- if settings.db %}
        create_app::<Self, Migrator>(mode, environment).await
        {% else %}
        create_app::<Self>(mode, environment).await
        {%- endif %}
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![
        {%- if settings.initializers.view_engine -%}
        Box::new(initializers::view_engine::ViewEngineInitializer)
        {%- endif -%}
        ])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
        {%- if settings.auth %}
            .add_route(controllers::auth::routes())
        {%- else %}
            .add_route(controllers::home::routes())
        {%- endif %}
    }

    {%- if settings.background %}
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
    {%- else %}
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
    {%- endif %} 
        {%- if settings.background %}
        queue.register(DownloadWorker::build(ctx)).await?;
        {%- endif %}
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }

    {%- if settings.db %}
    async fn truncate(db: &DatabaseConnection) -> Result<()> {
        truncate_table(db, users::Entity).await?;
        Ok(())
    }

    async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(db, &base.join("users.yaml").display().to_string()).await?;
        Ok(())
    }
    {%- endif %}
}
