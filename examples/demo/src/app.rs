use std::path::Path;

use async_trait::async_trait;
use axum::Router as AxumRouter;
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
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme, HttpBuilder, HttpAuthScheme},
    Modify, OpenApi,
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_redoc::{Redoc, Servable};
use utoipa_scalar::{Scalar, Servable as ScalarServable};
use utoipa_swagger_ui::SwaggerUi;

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
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        let initializers: Vec<Box<dyn Initializer>> = vec![
            Box::new(initializers::axum_session::AxumSessionInitializer),
            Box::new(initializers::view_engine::ViewEngineInitializer),
            Box::new(initializers::hello_view_engine::HelloViewEngineInitializer),
        ];

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

    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        // Serving the OpenAPI doc
        #[derive(OpenApi)]
        #[openapi(modifiers(&SecurityAddon),
            info(
            title = "Loco Demo",
            description = "This app is a kitchensink for various capabilities and examples of the [Loco](https://loco.rs) project."
        ))]
        struct ApiDoc;

        // TODO set the jwt token location
        // let auth_location = ctx.config.auth.as_ref();

        struct SecurityAddon;
        impl Modify for SecurityAddon {
            fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
                if let Some(components) = openapi.components.as_mut() {
                    components.add_security_schemes_from_iter([
                        (
                            "jwt_token",
                            SecurityScheme::Http(
                                HttpBuilder::new()
                                    .scheme(HttpAuthScheme::Bearer)
                                    .bearer_format("JWT")
                                    .build(),
                            ),
                        ),
                        (
                            "api_key",
                            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("apikey"))),
                        ),
                    ]);
                }
            }
        }

        let (_, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
            .merge(controllers::auth::api_routes())
            .merge(controllers::responses::api_routes())
            .split_for_parts();

        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()))
            .merge(Redoc::with_url("/redoc", api.clone()))
            .merge(Scalar::with_url("/scalar", api));

        Ok(router)
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
