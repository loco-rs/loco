//! # Application Bootstrapping and Logic
//! This module contains functions and structures for bootstrapping and running
//! your application.
//!
//! # Example
//!
//! This example hows manually bootstrapping the webserver.
//!
//! ```rust
//! # #[cfg(feature = "with-db")] {
//! use async_trait::async_trait;
//! use loco_rs::{
//!     app::{AppContext, Hooks},
//!     controller::AppRoutes,
//!     boot::{self, StartMode},
//!     worker::Processor,
//!     task::Tasks,
//!     Result,
//! };
//! use sea_orm::DatabaseConnection;
//! use std::path::Path;
//!
//! /// this code block should be taken from the sea_orm migration model.
//! pub use sea_orm_migration::prelude::*;
//! pub struct Migrator;
//! pub struct App;
//! #[async_trait::async_trait]
//! impl MigratorTrait for Migrator {
//!     fn migrations() -> Vec<Box<dyn MigrationTrait>> {
//!         vec![]
//!     }
//! }
//!
//! #[async_trait]
//! impl Hooks for App {
//!     fn routes() -> AppRoutes {
//!         AppRoutes::with_default_routes()
//!     }
//!
//!     fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {}
//!
//!     fn register_tasks(tasks: &mut Tasks) {}
//!
//!     async fn truncate(db: &DatabaseConnection) -> Result<()> {
//!         Ok(())
//!     }
//!
//!     async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {
//!         Ok(())
//!     }
//! }
//!
//! let boot_result = boot::create_app::<App, Migrator>(StartMode::ServerAndWorker, "development");
//! }
//! ```
use std::{collections::BTreeMap, str::FromStr};

use axum::Router;
#[cfg(feature = "with-db")]
use sea_orm_migration::MigratorTrait;
use tracing::trace;

#[cfg(feature = "with-db")]
use crate::db;
use crate::{
    app::{AppContext, Hooks},
    config::{self, Config},
    controller::ListRoutes,
    environment::Environment,
    errors::Error,
    logger,
    mailer::{EmailSender, MailerWorker},
    redis,
    task::Tasks,
    worker::{self, AppWorker, Pool, Processor, RedisConnectionManager, DEFAULT_QUEUES},
    Result,
};

/// Represents the application startup mode.
pub enum StartMode {
    /// Run the application as a server only. when running web server only,
    /// workers job will not handle.
    ServerOnly,
    /// Run the application web server and the worker in the same process.
    ServerAndWorker,
    /// Pulling job worker and execute them
    WorkerOnly,
}
pub struct BootResult {
    /// Application Context
    pub app_context: AppContext,
    /// Web server routes
    pub router: Option<Router>,
    /// worker processor
    pub processor: Option<Processor>,
}

/// Runs the application based on the provided `BootResult`.
///
/// This function is responsible for starting the application, including the
/// server and/or workers.
///
/// # Errors
///
/// When could not initialize the application.
pub async fn start(boot: BootResult) -> Result<()> {
    let BootResult {
        router,
        processor,
        app_context,
    } = boot;
    tracing::warn!("starting in {}", app_context.environment);
    match (router, processor) {
        (Some(router), Some(processor)) => {
            tokio::spawn(async move {
                if let Err(err) = process(processor).await {
                    tracing::error!("Error in processing: {:?}", err);
                } else {
                    tracing::warn!("Workers online");
                }
            });
            serve(router, &app_context.config).await?;
        }
        (Some(router), None) => {
            serve(router, &app_context.config).await?;
        }
        (None, Some(processor)) => {
            tracing::warn!("workers online");
            process(processor).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn process(processor: Processor) -> Result<()> {
    processor.run().await;
    Ok(())
}

/// Run task
///
/// # Errors
///
/// When running could not run the task.
pub async fn run_task<H: Hooks>(
    app_context: &AppContext,
    task: Option<&String>,
    vars: &BTreeMap<String, String>,
) -> Result<()> {
    let mut tasks = Tasks::default();
    H::register_tasks(&mut tasks);

    if let Some(task) = task {
        tasks.run(app_context, task, vars).await?;
    } else {
        let list = tasks.list();
        for item in &list {
            println!("{}\t\t[{}]", item.name, item.detail);
        }
    }
    Ok(())
}

/// Represents commands for handling database-related operations.
#[derive(Debug)]
pub enum RunDbCommand {
    /// Apply pending migrations.
    Migrate,
    /// Drop all tables, then reapply all migrations.
    Reset,
    /// Check the status of all migrations.
    Status,
    /// Generate entity.
    Entities,
    /// Truncate tables, by executing the implementation in [`Hooks::seed`]
    /// (without dropping).
    Truncate,
}

#[cfg(feature = "with-db")]
/// Handles database commands.
///
/// # Errors
///
/// Return an error when the given command fails. mostly return
/// [`sea_orm::DbErr`]
pub async fn run_db<H: Hooks, M: MigratorTrait>(
    app_context: &AppContext,
    cmd: RunDbCommand,
) -> Result<()> {
    match cmd {
        RunDbCommand::Migrate => {
            tracing::warn!("migrate:");
            let _ = db::migrate::<M>(&app_context.db).await;
        }
        RunDbCommand::Reset => {
            tracing::warn!("reset:");
            let _ = db::reset::<M>(&app_context.db).await;
        }
        RunDbCommand::Status => {
            tracing::warn!("status:");
            let _ = db::status::<M>(&app_context.db).await;
        }
        RunDbCommand::Entities => {
            tracing::warn!("entities:");
            tracing::warn!("{}", db::entities::<M>(&app_context.db)?);
        }
        RunDbCommand::Truncate => {
            tracing::warn!("truncate:");
            H::truncate(&app_context.db).await?;
        }
    }
    Ok(())
}

/// Starts the server using the provided [`Router`] and [`Config`].
async fn serve(app: Router, config: &Config) -> Result<()> {
    println!("start on port {}", config.server.port);
    axum::Server::bind(&format!("0.0.0.0:{}", config.server.port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

/// Initializes the application context by loading configuration and
/// establishing connections.
///
/// # Errors
/// When has an error to create DB connection.
pub async fn create_context(environment: &str) -> Result<AppContext> {
    let environment = Environment::from_str(environment)
        .unwrap_or_else(|_| Environment::Any(environment.to_string()));
    let config = environment.load()?;
    if config.logger.enable {
        logger::init(&config.logger);
    }
    #[cfg(feature = "with-db")]
    let db = db::connect(&config.database).await?;

    let mailer = if let Some(cfg) = config.mailer.as_ref() {
        create_mailer(cfg)?
    } else {
        None
    };

    let redis = connect_redis(&config).await;
    Ok(AppContext {
        environment,
        #[cfg(feature = "with-db")]
        db,
        redis,
        config,
        mailer,
    })
}

#[cfg(feature = "with-db")]
/// Creates an application based on the specified mode and environment.
///
/// # Errors
///
/// When could not create the application
pub async fn create_app<H: Hooks, M: MigratorTrait>(
    mode: StartMode,
    environment: &str,
) -> Result<BootResult> {
    let app_context = create_context(environment).await?;
    db::converge::<H, M>(&app_context.db, &app_context.config.database).await?;

    if let Some(pool) = &app_context.redis {
        redis::converge(pool, &app_context.config.redis).await?;
    }

    run_app::<H>(&mode, app_context)
}

#[cfg(not(feature = "with-db"))]
pub async fn create_app<H: Hooks>(mode: StartMode, environment: &str) -> Result<BootResult> {
    let app_context = create_context(environment).await?;

    if let Some(pool) = &app_context.redis {
        redis::converge(pool, &app_context.config.redis).await?;
    }

    run_app::<H>(&mode, app_context)
}

/// Run the application with the  given mode
/// # Errors
///
/// When could not create the application
pub fn run_app<H: Hooks>(mode: &StartMode, app_context: AppContext) -> Result<BootResult> {
    match mode {
        StartMode::ServerOnly => {
            let app = H::routes().to_router(app_context.clone())?;
            Ok(BootResult {
                app_context,
                router: Some(app),
                processor: None,
            })
        }
        StartMode::ServerAndWorker => {
            let processor = create_processor::<H>(&app_context)?;
            let app = H::routes().to_router(app_context.clone())?;
            Ok(BootResult {
                app_context,
                router: Some(app),
                processor: Some(processor),
            })
        }
        StartMode::WorkerOnly => {
            let processor = create_processor::<H>(&app_context)?;
            Ok(BootResult {
                app_context,
                router: None,
                processor: Some(processor),
            })
        }
    }
}
/// Creates and configures a [`Processor`] for handling worker tasks.
fn create_processor<H: Hooks>(app_context: &AppContext) -> Result<Processor> {
    let queues = worker::get_queues(&app_context.config.workers.queues);
    trace!(
        queues = ?queues,
        "registering queues (merged config and default)"
    );
    let mut p = if let Some(redis) = &app_context.redis {
        Processor::new(
            redis.clone(),
            // TODO(config)
            DEFAULT_QUEUES
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        )
    } else {
        return Err(Error::Message(
            "redis is missing, cannot initialize workers".to_string(),
        ));
    };

    p.register(MailerWorker::build(app_context));
    H::connect_workers(&mut p, app_context);

    trace!("done registering workers and queues");
    Ok(p)
}

#[must_use]
pub fn list_endpoints<H: Hooks>() -> Vec<ListRoutes> {
    H::routes().collect()
}

/// Initializes an [`EmailSender`] based on the mailer configuration settings
/// ([`config::Mailer`]).
fn create_mailer(config: &config::Mailer) -> Result<Option<EmailSender>> {
    if let Some(smtp) = config.smtp.as_ref() {
        if smtp.enable {
            return Ok(Some(EmailSender::smtp(smtp)?));
        }
    }
    Ok(None)
}

/// Establishes a connection to a Redis server based on the provided
/// configuration settings.
async fn connect_redis(config: &Config) -> Option<Pool<RedisConnectionManager>> {
    if let Some(redis) = &config.redis {
        let manager = RedisConnectionManager::new(redis.uri.clone()).unwrap();
        let redis = Pool::builder().build(manager).await.unwrap();
        Some(redis)
    } else {
        None
    }
}
