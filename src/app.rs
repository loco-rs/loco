//! This module contains the core components and traits for building a web
//! server application.
cfg_if::cfg_if! {
    if #[cfg(feature = "with-db")] {
        use std::path::Path;
        use sea_orm::DatabaseConnection;
    } else {}

}
use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::FromRef, Router as AxumRouter};
use axum_extra::extract::cookie::Key;

#[cfg(feature = "channels")]
use crate::controller::channels::AppChannels;
#[cfg(feature = "oauth2")]
use crate::oauth2_store::OAuth2ClientStore;
use crate::{
    boot::{BootResult, ServeParams, StartMode},
    config::{self, Config},
    controller::AppRoutes,
    environment::Environment,
    mailer::EmailSender,
    storage::Storage,
    task::Tasks,
    worker::{Pool, Processor, RedisConnectionManager},
    Result,
};

/// Represents the application context for a web server.
///
/// This struct encapsulates various components and configurations required by
/// the web server to operate. It is typically used to store and manage shared
/// resources and settings that are accessible throughout the application's
/// lifetime.
#[derive(Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct AppContext {
    /// The environment in which the application is running.
    pub environment: Environment,
    #[cfg(feature = "with-db")]
    /// A database connection used by the application.    
    pub db: DatabaseConnection,
    /// An optional connection pool for Redis, for worker tasks
    pub redis: Option<Pool<RedisConnectionManager>>,
    /// Configuration settings for the application
    pub config: Config,
    /// An optional email sender component that can be used to send email.
    pub mailer: Option<EmailSender>,
    // Ab optional storage instance for the application
    pub storage: Option<Arc<Storage>>,
    // An optional oauth2 client
    #[cfg(feature = "oauth2")]
    pub oauth2: Option<Arc<OAuth2ClientStore>>,
    // An optional key for use with Signed and/or Private jars
    #[cfg(feature = "oauth2")]
    pub key: Key,
}

/// A trait that defines hooks for customizing and extending the behavior of a
/// web server application.
///
/// Users of the web server application should implement this trait to customize
/// the application's routing, worker connections, task registration, and
/// database actions according to their specific requirements and use cases.
#[async_trait]
pub trait Hooks {
    /// Defines the composite app version
    #[must_use]
    fn app_version() -> String {
        "dev".to_string()
    }
    /// Defines the crate name
    ///
    /// Example
    /// ```rust
    /// fn app_name() -> &'static str {
    ///     env!("CARGO_CRATE_NAME")
    /// }
    /// ```
    fn app_name() -> &'static str;

    /// Initializes and boots the application based on the specified mode and
    /// environment.
    ///
    /// The boot initialization process may vary depending on whether a DB
    /// migrator is used or not.
    ///
    /// # Examples
    ///
    /// With DB:
    /// ```rust,ignore
    /// async fn boot(mode: StartMode, environment: &str) -> Result<BootResult> {
    ///     create_app::<Self, Migrator>(mode, environment).await
    /// }
    /// ````
    ///
    /// Without DB:
    /// ```rust,ignore
    /// async fn boot(mode: StartMode, environment: &str) -> Result<BootResult> {
    ///     create_app::<Self>(mode, environment).await
    /// }
    /// ````
    ///
    ///
    /// # Errors
    /// Could not boot the application
    async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult>;

    /// Start serving the Axum web application on the specified address and
    /// port.
    ///
    /// # Returns
    /// A Result indicating success () or an error if the server fails to start.
    async fn serve(app: AxumRouter, server_config: ServeParams) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(&format!(
            "{}:{}",
            server_config.binding, server_config.port
        ))
        .await?;

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Override and return `Ok(true)` to provide an alternative logging and
    /// tracing stack of your own.
    /// When returning `Ok(true)`, Loco will *not* initialize its own logger,
    /// so you should set up a complete tracing and logging stack.
    ///
    /// # Errors
    /// If fails returns an error
    fn init_logger(_config: &config::Config, _env: &Environment) -> Result<bool> {
        Ok(false)
    }

    /// Invoke this function after the Loco routers have been constructed. This
    /// function enables you to configure custom Axum logics, such as layers,
    /// that are compatible with Axum.
    ///
    /// # Errors
    /// Axum router error
    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router)
    }

    /// Provide a list of initializers
    /// An initializer can be used to seamlessly add functionality to your app
    /// or to initialize some aspects of it.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Calling the function before run the app
    /// You can now code some custom loading of resources or other things before
    /// the app runs
    async fn before_run(_app_context: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Defines the application's routing configuration.
    fn routes(_ctx: &AppContext) -> AppRoutes;

    /// Defines the storage configuration for the application
    async fn storage(
        _config: &config::Config,
        _environment: &Environment,
    ) -> Result<Option<Storage>> {
        Ok(None)
    }
    /// Defines the OAuth2 configuration for the application
    #[cfg(feature = "oauth2")]
    async fn oauth2(
        _config: &config::Config,
        _environment: &Environment,
    ) -> Result<Option<OAuth2ClientStore>> {
        Ok(None)
    }
    #[cfg(feature = "channels")]
    /// Register channels endpoints to the application routers
    fn register_channels(_ctx: &AppContext) -> AppChannels;

    /// Connects custom workers to the application using the provided
    /// [`Processor`] and [`AppContext`].
    fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext);

    /// Registers custom tasks with the provided [`Tasks`] object.
    fn register_tasks(tasks: &mut Tasks);

    /// Truncates the database as required. Users should implement this
    /// function. The truncate controlled from the [`crate::config::Database`]
    /// by changing dangerously_truncate to true (default false).
    /// Truncate can be useful when you want to truncate the database before any
    /// test.        
    #[cfg(feature = "with-db")]
    async fn truncate(db: &DatabaseConnection) -> Result<()>;

    /// Seeds the database with initial data.    
    #[cfg(feature = "with-db")]
    async fn seed(db: &DatabaseConnection, path: &Path) -> Result<()>;
}

/// An initializer.
/// Initializers should be kept in `src/initializers/`
#[async_trait]
pub trait Initializer: Sync + Send {
    /// The initializer name or identifier
    fn name(&self) -> String;

    /// Occurs after the app's `before_run`.
    /// Use this to for one-time initializations, load caches, perform web
    /// hooks, etc.
    async fn before_run(&self, _app_context: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Occurs after the app's `after_routes`.
    /// Use this to compose additional functionality and wire it into an Axum
    /// Router
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router)
    }
}
// implementing FromRef is required here so we can extract substate in Axum
// read more here: https://docs.rs/axum/latest/axum/extract/trait.FromRef.html
#[cfg(feature = "oauth2")]
impl FromRef<AppContext> for Key {
    fn from_ref(state: &AppContext) -> Self {
        state.key.clone()
    }
}
