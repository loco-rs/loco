//! This module contains the core components and traits for building a web
//! server application.
cfg_if::cfg_if! {
    if #[cfg(feature = "with-db")] {
        use std::path::Path;
        use sea_orm::DatabaseConnection;
        use crate::Result;
    } else {}
}

use async_trait::async_trait;

use crate::{
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    mailer::EmailSender,
    task::Tasks,
    worker::{Pool, Processor, RedisConnectionManager},
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
}

/// A trait that defines hooks for customizing and extending the behavior of a
/// web server application.
///
/// Users of the web server application should implement this trait to customize
/// the application's routing, worker connections, task registration, and
/// database actions according to their specific requirements and use cases.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "with-db")] {
/// use loco_rs::{
///     app::{AppContext, Hooks},
///     controller::AppRoutes,
///     db::{self, truncate_table},
///     task::Tasks,
///     worker::{AppWorker, Processor},
///     Result,
/// };
/// use sea_orm::DatabaseConnection;
/// use std::path::Path;
/// use async_trait::async_trait;
///
/// pub struct App;
/// #[async_trait]
/// impl Hooks for App {
///     fn routes() -> AppRoutes {
///         AppRoutes::with_default_routes()
///             // .add_route(controllers::notes::routes())
///             // .add_route(controllers::auth::routes())
///     }
///
///     fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {
///         // p.register(DownloadWorker::build(ctx));
///     }
///
///     fn register_tasks(tasks: &mut Tasks) {
///         // tasks.register(UserReport);
///     }
///
///     async fn truncate(db: &DatabaseConnection) -> Result<()> {
///         // truncate_table(db, users::Entity).await?;
///         // truncate_table(db, notes::Entity).await?;
///         Ok(())
///     }
///
///     async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {
///         // db::seed::<users::ActiveModel>(db, &base.join("users.yaml").display().to_string()).await?;
///         Ok(())
///     }
/// }
/// }
/// ```
#[async_trait]
pub trait Hooks {
    /// Defines the application's routing configuration.
    fn routes() -> AppRoutes;
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
