//! # Application Bootstrapping and Logic
//! This module contains functions and structures for bootstrapping and running
//! your application.
use std::path::PathBuf;

use axum::Router;
#[cfg(feature = "with-db")]
use sea_orm_migration::MigratorTrait;
use tracing::{info, trace, warn};

#[cfg(feature = "with-db")]
use crate::db;
use crate::{
    app::{AppContextTrait, Hooks},
    banner::print_banner,
    config::{self, Config},
    controller::ListRoutes,
    environment::Environment,
    errors::Error,
    mailer::{EmailSender, MailerWorker},
    redis,
    scheduler::{self, Scheduler},
    task::{self, Tasks},
    worker::{self, AppWorker, Pool, Processor, RedisConnectionManager},
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
pub struct BootResult<AC: AppContextTrait> {
    /// Application Context
    pub app_context: AC,
    /// Web server routes
    pub router: Option<Router>,
    /// worker processor
    pub processor: Option<Processor>,
}

/// Configuration structure for serving an application.
pub struct ServeParams {
    /// The port number on which the server will listen for incoming
    /// connections.
    pub port: i32,
    /// The network address to which the server will bind. It specifies the
    /// interface to listen on.
    pub binding: String,
}

/// Runs the application based on the provided `BootResult`.
///
/// This function is responsible for starting the application, including the
/// server and/or workers.
///
/// # Errors
///
/// When could not initialize the application.
pub async fn start<AC: AppContextTrait, H: Hooks<AC>>(
    boot: BootResult<AC>,
    server_config: ServeParams,
) -> Result<()> {
    print_banner(&boot, &server_config);

    let BootResult {
        router,
        processor,
        app_context: _,
    } = boot;

    match (router, processor) {
        (Some(router), Some(processor)) => {
            tokio::spawn(async move {
                if let Err(err) = process(processor).await {
                    tracing::error!("Error in processing: {:?}", err);
                }
            });
            H::serve(router, server_config).await?;
        }
        (Some(router), None) => {
            H::serve(router, server_config).await?;
        }
        (None, Some(processor)) => {
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
pub async fn run_task<AC: AppContextTrait, H: Hooks<AC>>(
    app_context: &AC,
    task: Option<&String>,
    vars: &task::Vars,
) -> Result<()> {
    let mut tasks = Tasks::default();
    H::register_tasks(&mut tasks);

    if let Some(task) = task {
        let task_span = tracing::span!(tracing::Level::DEBUG, "task", task,);
        let _guard = task_span.enter();
        tasks.run(app_context, task, vars).await?;
    } else {
        let list = tasks.list();
        for item in &list {
            println!("{:<30}[{}]", item.name, item.detail);
        }
    }
    Ok(())
}

/// Runs the scheduler with the given configuration and context. in case if list args is true
/// prints scheduler job configuration
///
/// This function initializes the scheduler, registers tasks through the provided [`Hooks`],
/// and executes the scheduler based on the specified configuration or context. The scheduler
/// continuously runs, managing and executing scheduled tasks until a signal is received to shut down.
/// Upon receiving this signal, the function gracefully shuts down all running tasks and exits safely.
///
/// # Errors
///
/// When running could not run the scheduler.
pub async fn run_scheduler<AC: AppContextTrait, H: Hooks<AC>>(
    app_context: &AC,
    config: Option<&PathBuf>,
    name: Option<String>,
    tag: Option<String>,
    list: bool,
) -> Result<()> {
    let mut tasks = Tasks::default();
    H::register_tasks(&mut tasks);

    let task_span = tracing::span!(tracing::Level::DEBUG, "scheduler_jobs");
    let _guard = task_span.enter();

    let scheduler = match config {
        Some(path) => Scheduler::from_config::<AC, H>(path, app_context.environment())?,
        None => {
            if let Some(config) = &app_context.config().scheduler {
                Scheduler::new::<AC, H>(config, app_context.environment())?
            } else {
                return Err(Error::Scheduler(scheduler::Error::Empty));
            }
        }
    };

    let scheduler = scheduler.by_spec(&scheduler::Spec { name, tag });
    if list {
        println!("{scheduler}");
        Ok(())
    } else {
        Ok(scheduler.run().await?)
    }
}

/// Represents commands for handling database-related operations.
#[derive(Debug)]
pub enum RunDbCommand {
    /// Apply pending migrations.
    Migrate,
    /// Run one or more down migrations.
    Down(u32),
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
pub async fn run_db<AC: AppContextTrait, H: Hooks<AC>, M: MigratorTrait>(
    app_context: &AC,
    cmd: RunDbCommand,
) -> Result<()> {
    match cmd {
        RunDbCommand::Migrate => {
            tracing::warn!("migrate:");
            db::migrate::<M>(app_context.db()).await?;
        }
        RunDbCommand::Down(steps) => {
            tracing::warn!("down:");
            db::down::<M>(app_context.db(), steps).await?;
        }
        RunDbCommand::Reset => {
            tracing::warn!("reset:");
            db::reset::<M>(app_context.db()).await?;
        }
        RunDbCommand::Status => {
            tracing::warn!("status:");
            db::status::<M>(app_context.db()).await?;
        }
        RunDbCommand::Entities => {
            tracing::warn!("entities:");

            tracing::warn!("{}", db::entities::<AC, M>(app_context).await?);
        }
        RunDbCommand::Truncate => {
            tracing::warn!("truncate:");
            H::truncate(app_context.db()).await?;
        }
    }
    Ok(())
}

/// Initializes the application context by loading configuration and
/// establishing connections.
///
/// # Errors
/// When has an error to create DB connection.
pub async fn create_context<AC: AppContextTrait, H: Hooks<AC>>(
    environment: &Environment,
) -> Result<AC> {
    let config = environment.load()?;

    if config.logger.pretty_backtrace {
        std::env::set_var("RUST_BACKTRACE", "1");
        warn!(
            "pretty backtraces are enabled (this is great for development but has a runtime cost \
             for production. disable with `logger.pretty_backtrace` in your config yaml)"
        );
    }

    let queue = connect_redis(&config).await;

    #[cfg(feature = "with-db")]
    let ctx = {
        let db = db::connect(&config.database).await?;
        AC::create(environment.clone(), config, db, queue)?
    };

    #[cfg(not(feature = "with-db"))]
    let ctx = AC::create(environment.clone(), config, queue)?;

    H::after_context(ctx).await
}

#[cfg(feature = "with-db")]
/// Creates an application based on the specified mode and environment.
///
/// # Errors
///
/// When could not create the application
pub async fn create_app<AC: AppContextTrait, H: Hooks<AC>, M: MigratorTrait>(
    mode: StartMode,
    environment: &Environment,
) -> Result<BootResult<AC>> {
    let app_context = create_context::<AC, H>(environment).await?;
    db::converge::<AC, H, M>(app_context.db(), &app_context.config().database).await?;

    if let Some(pool) = app_context.queue() {
        redis::converge(pool, &app_context.config().queue).await?;
    }

    run_app::<AC, H>(&mode, app_context).await
}

#[cfg(not(feature = "with-db"))]
pub async fn create_app<AC: AppContextTrait, H: Hooks<AC>>(
    mode: StartMode,
    environment: &Environment,
) -> Result<BootResult> {
    let app_context = create_context::<AC, H>(environment).await?;

    if let Some(pool) = &app_context.queue {
        redis::converge(pool, &app_context.config.queue).await?;
    }

    run_app::<AC, H>(&mode, app_context).await
}

/// Run the application with the  given mode
/// # Errors
///
/// When could not create the application
pub async fn run_app<AC: AppContextTrait, H: Hooks<AC>>(
    mode: &StartMode,
    app_context: AC,
) -> Result<BootResult<AC>> {
    H::before_run(&app_context).await?;
    let initializers = H::initializers(&app_context).await?;
    info!(initializers = ?initializers.iter().map(|init| init.name()).collect::<Vec<_>>().join(","), "initializers loaded");
    for initializer in &initializers {
        initializer.before_run(&app_context).await?;
    }
    match mode {
        StartMode::ServerOnly => {
            let app = H::before_routes(&app_context).await?;
            let app = H::routes(&app_context).to_router(app_context.clone(), app)?;
            let mut router = H::after_routes(app, &app_context).await?;
            for initializer in &initializers {
                router = initializer.after_routes(router, &app_context).await?;
            }

            Ok(BootResult {
                app_context,
                router: Some(router),
                processor: None,
            })
        }
        StartMode::ServerAndWorker => {
            let processor = create_processor::<AC, H>(&app_context)?;
            let app = H::before_routes(&app_context).await?;
            let app = H::routes(&app_context).to_router(app_context.clone(), app)?;
            let mut router = H::after_routes(app, &app_context).await?;
            for initializer in &initializers {
                router = initializer.after_routes(router, &app_context).await?;
            }
            Ok(BootResult {
                app_context,
                router: Some(router),
                processor: Some(processor),
            })
        }
        StartMode::WorkerOnly => {
            let processor = create_processor::<AC, H>(&app_context)?;
            Ok(BootResult {
                app_context,
                router: None,
                processor: Some(processor),
            })
        }
    }
}
/// Creates and configures a [`Processor`] for handling worker tasks.
fn create_processor<AC: AppContextTrait, H: Hooks<AC>>(app_context: &AC) -> Result<Processor> {
    let queues = worker::get_queues(&app_context.config().workers.queues);
    trace!(
        queues = ?queues,
        "registering queues (merged config and default)"
    );
    let mut p = if let Some(queue) = app_context.queue() {
        Processor::new(queue.clone(), queues)
    } else {
        return Err(Error::Message(
            "queue is missing, cannot initialize workers".to_string(),
        ));
    };

    p.register(MailerWorker::build(app_context));
    H::connect_workers(&mut p, app_context);

    trace!("done registering workers and queues");
    Ok(p)
}

#[must_use]
pub fn list_endpoints<AC: AppContextTrait, H: Hooks<AC>>(ctx: &AC) -> Vec<ListRoutes<AC>> {
    H::routes(ctx).collect()
}

/// Initializes an [`EmailSender`] based on the mailer configuration settings
/// ([`config::Mailer`]).
pub(crate) fn create_mailer(config: &config::Mailer) -> Result<Option<EmailSender>> {
    if config.stub {
        return Ok(Some(EmailSender::stub()));
    }
    if let Some(smtp) = config.smtp.as_ref() {
        if smtp.enable {
            return Ok(Some(EmailSender::smtp(smtp)?));
        }
    }
    Ok(None)
}

#[allow(clippy::missing_panics_doc)]
/// Establishes a connection to a Redis server based on the provided
/// configuration settings.
// TODO: Refactor to eliminate unwrapping and instead return an appropriate
// error type.
pub async fn connect_redis(config: &Config) -> Option<Pool<RedisConnectionManager>> {
    if config.workers.mode == config::WorkerMode::BackgroundQueue {
        if let Some(redis) = &config.queue {
            let manager = RedisConnectionManager::new(redis.uri.clone()).unwrap();
            let redis = Pool::builder().build(manager).await.unwrap();
            return Some(redis);
        }
    }
    None
}
