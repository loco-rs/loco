use std::{collections::BTreeMap, path::Path, sync::Arc};

use async_trait::async_trait;
use loco_extras;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    oauth2_store::{oauth2_grant::OAuth2ClientGrantEnum, OAuth2ClientStore},
    storage::{self, Storage},
    task::Tasks,
    worker::{AppWorker, Processor},
    Result,
};
use migration::Migrator;
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

use crate::{
    controllers, initializers,
    models::_entities::{notes, users},
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

    async fn initializers(ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        let mut initializers: Vec<Box<dyn Initializer>> = vec![
            Box::new(initializers::axum_session::AxumSessionInitializer),
            Box::new(initializers::view_engine::ViewEngineInitializer),
            Box::new(initializers::hello_view_engine::HelloViewEngineInitializer),
        ];

        if ctx.environment != Environment::Test {
            initializers.push(Box::new(
                loco_extras::initializers::prometheus::AxumPrometheusInitializer,
            ));
        }

        Ok(initializers)
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::notes::routes())
            .add_route(controllers::auth::routes())
            .add_route(controllers::mysession::routes())
            .add_route(controllers::dashboard::routes())
            .add_route(controllers::user::routes())
            .add_route(controllers::upload::routes())
            .add_route(controllers::oauth2::routes())
    }

    async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment).await
    }

    async fn storage(
        _config: &Config,
        environment: &Environment,
    ) -> Result<Option<storage::Storage>> {
        let store = if environment == &Environment::Test {
            storage::drivers::mem::new()
        } else {
            storage::drivers::local::new_with_prefix("storage-uploads").map_err(Box::from)?
        };

        let storage = Storage::single(store);
        return Ok(Some(storage));
    }

    async fn oauth2(
        config: &Config,
        environment: &Environment,
    ) -> Result<Option<OAuth2ClientStore>> {
        if environment != &Environment::Test && environment != &Environment::Development {
            return Ok(None);
        }
        let oauth2_config = config
            .oauth2
            .clone()
            .ok_or(loco_rs::Error::string("Missing configuration for oauth2"))?;
        let authorization_code_grants = oauth2_config.authorization_code;
        let mut clients = BTreeMap::new();
        for grant in authorization_code_grants {
            let client =
                loco_rs::oauth2_store::grants::authorization_code::AuthorizationCodeClient::new(
                    grant.client_credentials,
                    grant.url_config,
                    None,
                )?;
            clients.insert(
                grant.provider_name,
                OAuth2ClientGrantEnum::AuthorizationCode(Arc::new(Mutex::new(client))),
            );
        }
        let store = OAuth2ClientStore::new(clients);
        Ok(Some(store))
    }

    fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {
        p.register(DownloadWorker::build(ctx));
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::user_report::UserReport);
        tasks.register(tasks::seed::SeedData);
        tasks.register(tasks::foo::Foo);
    }

    async fn truncate(db: &DatabaseConnection) -> Result<()> {
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
