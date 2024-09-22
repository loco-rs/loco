//! This module contains the core components and traits for building a web
//! server application.
cfg_if::cfg_if! {
    if #[cfg(feature = "with-db")] {
        use std::path::Path;
        use sea_orm::DatabaseConnection;
    } else {}

}
use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use axum::Router as AxumRouter;

#[cfg(feature = "channels")]
use crate::controller::channels::AppChannels;
use crate::{
    boot::{create_mailer, BootResult, ServeParams, StartMode},
    cache::{self, Cache},
    config::{self, Config},
    controller::AppRoutes,
    environment::Environment,
    mailer::EmailSender,
    storage::{self, Storage},
    task::Tasks,
    worker::{Pool, Processor, RedisConnectionManager},
    Result,
};

/// Object-safe trait for representing application context needed
/// by the web server to operate.
///
/// See [AppContextTrait] for more complete documentation on
/// application context.
pub trait Context: Send + Sync + 'static {
    fn environment(&self) -> &Environment;
    #[cfg(feature = "with-db")]
    fn db(&self) -> &DatabaseConnection;
    fn queue(&self) -> &Option<Pool<RedisConnectionManager>>;
    fn config(&self) -> &Config;
    fn mailer(&self) -> &Option<EmailSender>;
    fn storage(&self) -> Arc<Storage>;
    fn cache(&self) -> Arc<cache::Cache>;
}

/// This trait defines the configuration required by the
/// web server to operate.
///
/// This trait along with [Context] should be implemented for any
/// struct used to represent the application context. A default implementation
/// is provided by the [AppContext] struct that can be used in your server.
///
/// ```rust,ignore
/// use loco_rs::{app::{AppContext, AppContextTrait, Context}};
///
/// #[derive(Default)]
/// struct LocalContext {
///     app_context: AppContext,
/// }
///
/// impl Context for LocalContext {
///     fn environment(&self) -> &Environment {
///         &self.app_context.environment
///     }
///
///     #[cfg(feature = "with-db")]
///     fn db(&self) -> &DatabaseConnection {
///         &self.app_context.db
///     }
///
///     fn queue(&self) -> &Option<Pool<RedisConnectionManager>> {
///         &self.app_context.queue
///     }
///
///     fn config(&self) -> &Config {
///         &self.app_context.config
///     }
///
///     fn mailer(&self) -> &Option<EmailSender> {
///         &self.app_context.mailer
///     }
///
///     fn storage(&self) -> Arc<Storage> {
///         self.app_context.storage.clone()
///     }
///
///     fn cache(&self) -> Arc<cache::Cache> {
///         self.app_context.cache.clone()
///     }
/// }
///
/// impl AppContextTrait for LocalContext {
///
///     #[cfg(feature = "with-db")]
///     fn create(
///         environment: Environment,
///         config: Config,
///         db: DatabaseConnection,
///         queue: Option<Pool<RedisConnectionManager>>,
///     ) -> Result<Self> {
///         let mailer = if let Some(cfg) = config.mailer.as_ref() {
///             create_mailer(cfg)?
///         } else {
///             None
///         };
///
///         Ok(LocalContext {
///             app_context: AppContext {
///                 environment,
///                 db,
///                 queue,
///                 storage: Storage::single(storage::drivers::null::new()).into(),
///                 cache: Cache::new(cache::drivers::null::new()).into(),
///                 config,
///                 mailer,
///             }
///         })
///     }
///
///
///
///     #[cfg(not(feature = "with-db"))]
///     fn create(
///         environment: Environment,
///         config: Config,
///         queue: Option<Pool<RedisConnectionManager>>,
///     ) -> Result<Self> {
///         let mailer = if let Some(cfg) = config.mailer.as_ref() {
///             create_mailer(cfg)?
///         } else {
///             None
///         };
///
///         Ok(LocalContext {
///             app_context: AppContext {
///                 environment,
///                 queue,
///                 storage: Storage::single(storage::drivers::null::new()).into(),
///                 cache: Cache::new(cache::drivers::null::new()).into(),
///                 config,
///                 mailer,
///             }
///         })
///     }
/// }
///
/// impl Hooks<LocalContext> for App {
///     .
///     .
///     .
/// }
/// ```
pub trait AppContextTrait: Context + Clone + Default {
    #[cfg(feature = "with-db")]
    fn create(
        environment: Environment,
        config: Config,
        db: DatabaseConnection,
        queue: Option<Pool<RedisConnectionManager>>,
    ) -> Result<Self>;
    #[cfg(not(feature = "with-db"))]
    fn create(
        environment: Environment,
        config: Config,
        queue: Option<Pool<RedisConnectionManager>>,
    ) -> Result<Self>;
}

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
    /// An optional connection pool for Queue, for worker tasks
    pub queue: Option<Pool<RedisConnectionManager>>,
    /// Configuration settings for the application
    pub config: Config,
    /// An optional email sender component that can be used to send email.
    pub mailer: Option<EmailSender>,
    // An optional storage instance for the application
    pub storage: Arc<Storage>,
    // Cache instance for the application
    pub cache: Arc<cache::Cache>,
}

impl Default for AppContext {
    fn default() -> Self {
        let environment = Environment::Test;
        #[cfg(feature = "with-db")]
        let db = DatabaseConnection::default();
        let config = environment
            .load()
            .expect("Failed to load config for test environment");

        AppContext {
            environment,
            #[cfg(feature = "with-db")]
            db,
            queue: None,
            storage: Storage::single(storage::drivers::null::new()).into(),
            cache: Cache::new(cache::drivers::null::new()).into(),
            config,
            mailer: None,
        }
    }
}

impl Context for AppContext {
    fn environment(&self) -> &Environment {
        &self.environment
    }

    #[cfg(feature = "with-db")]
    fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    fn queue(&self) -> &Option<Pool<RedisConnectionManager>> {
        &self.queue
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn mailer(&self) -> &Option<EmailSender> {
        &self.mailer
    }

    fn storage(&self) -> Arc<Storage> {
        self.storage.clone()
    }

    fn cache(&self) -> Arc<cache::Cache> {
        self.cache.clone()
    }
}

impl AppContextTrait for AppContext {
    #[cfg(feature = "with-db")]
    fn create(
        environment: Environment,
        config: Config,
        db: DatabaseConnection,
        queue: Option<Pool<RedisConnectionManager>>,
    ) -> Result<Self> {
        let mailer = if let Some(cfg) = config.mailer.as_ref() {
            create_mailer(cfg)?
        } else {
            None
        };

        Ok(AppContext {
            environment,
            db,
            queue,
            storage: Storage::single(storage::drivers::null::new()).into(),
            cache: Cache::new(cache::drivers::null::new()).into(),
            config,
            mailer,
        })
    }

    #[cfg(not(feature = "with-db"))]
    fn create(
        environment: Environment,
        config: Config,
        queue: Option<Pool<RedisConnectionManager>>,
    ) -> Result<Self> {
        let mailer = if let Some(cfg) = config.mailer.as_ref() {
            create_mailer(cfg)?
        } else {
            None
        };

        Ok(AppContext {
            environment,
            queue,
            storage: Storage::single(storage::drivers::null::new()).into(),
            cache: Cache::new(cache::drivers::null::new()).into(),
            config,
            mailer,
        })
    }
}

/// A trait that defines hooks for customizing and extending the behavior of a
/// web server application.
///
/// Users of the web server application should implement this trait to customize
/// the application's routing, worker connections, task registration, and
/// database actions according to their specific requirements and use cases.
#[async_trait]
pub trait Hooks<AC: AppContextTrait> {
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
    async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult<AC>>;

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

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

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

    /// Returns the initial Axum router for the application, allowing the user
    /// to control the construction of the Axum router. This is where a fallback
    /// handler can be installed before middleware or other routes are added.
    ///
    /// # Errors
    /// Return an [`Result`] when the router could not be created
    async fn before_routes(_ctx: &AC) -> Result<AxumRouter<AC>> {
        Ok(AxumRouter::new())
    }

    /// Invoke this function after the Loco routers have been constructed. This
    /// function enables you to configure custom Axum logics, such as layers,
    /// that are compatible with Axum.
    ///
    /// # Errors
    /// Axum router error
    async fn after_routes(router: AxumRouter, _ctx: &AC) -> Result<AxumRouter> {
        Ok(router)
    }

    /// Provide a list of initializers
    /// An initializer can be used to seamlessly add functionality to your app
    /// or to initialize some aspects of it.
    async fn initializers(_ctx: &AC) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Calling the function before run the app
    /// You can now code some custom loading of resources or other things before
    /// the app runs
    async fn before_run(_app_context: &AC) -> Result<()> {
        Ok(())
    }

    /// Defines the application's routing configuration.
    fn routes(_ctx: &AC) -> AppRoutes<AC>;

    // Provides the options to change Loco [`AppContext`] after initialization.
    async fn after_context(ctx: AC) -> Result<AC> {
        Ok(ctx)
    }

    #[cfg(feature = "channels")]
    /// Register channels endpoints to the application routers
    fn register_channels(_ctx: &AC) -> AppChannels;

    /// Connects custom workers to the application using the provided
    /// [`Processor`] and [`AppContext`].
    fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AC);

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
// <snip id="initializers-trait">
pub trait Initializer: Sync + Send {
    /// The initializer name or identifier
    fn name(&self) -> String;

    /// Occurs after the app's `before_run`.
    /// Use this to for one-time initializations, load caches, perform web
    /// hooks, etc.
    async fn before_run(&self, _app_context: &dyn Context) -> Result<()> {
        Ok(())
    }

    /// Occurs after the app's `after_routes`.
    /// Use this to compose additional functionality and wire it into an Axum
    /// Router
    async fn after_routes(&self, router: AxumRouter, _ctx: &dyn Context) -> Result<AxumRouter> {
        Ok(router)
    }
}
// </snip>
