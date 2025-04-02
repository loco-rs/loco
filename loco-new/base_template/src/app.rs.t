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
    config::Config,
    controller::AppRoutes,
    {%- if settings.auth %}
    db::{self, truncate_table},
    {%- endif %}
    environment::Environment,
    task::Tasks,
    Result,
};
{%- if settings.db %}
use migration::Migrator;
{%- endif %}

#[allow(unused_imports)]
use crate::{
    controllers ,tasks
    {%- if settings.initializers -%}
    , initializers
    {%- endif %} 
    {%- if settings.auth %}
    , models::_entities::users
    {%- endif %}
    {%- if settings.background %}
    , workers::downloader::DownloadWorker
    {%- endif %}
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

    async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult> {
        {%- if settings.db %}
        create_app::<Self, Migrator>(mode, environment, config).await
        {% else %}
        create_app::<Self>(mode, environment, config).await
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

    {%- if settings.auth %}
    async fn truncate(ctx: &AppContext) -> Result<()> {
    {%- else %}
    async fn truncate(_ctx: &AppContext) -> Result<()> {
    {%- endif %} 
        {%- if settings.auth %}
        truncate_table(&ctx.db, users::Entity).await?;
        {%- endif %}
        Ok(())
    }

    {%- if settings.auth %}
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
    {%- else %} 
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
    {%- endif %} 
        {%- if settings.auth %}
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string()).await?;
        {%- endif %}
        Ok(())
    }
    {%- endif %}
}
