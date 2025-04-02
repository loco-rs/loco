//! # Application Bootstrapping and Logic
//! This module contains functions and structures for bootstrapping and running
//! your application.
use std::{
    env,
    path::{Path, PathBuf},
};

use axum::Router;
#[cfg(feature = "with-db")]
use sea_orm_migration::MigratorTrait;
use tokio::{select, signal, task::JoinHandle};
use tracing::{debug, error, info, warn};

#[cfg(feature = "with-db")]
use crate::db;
use crate::{
    app::{AppContext, Hooks, Initializer},
    banner::print_banner,
    bgworker, cache,
    config::{self, Config, WorkerMode},
    controller::ListRoutes,
    env_vars,
    environment::Environment,
    errors::Error,
    mailer::{EmailSender, MailerWorker},
    prelude::BackgroundWorker,
    scheduler::{self, Scheduler},
    storage::{self, Storage},
    task::{self, Tasks},
    Result,
};

/// Represents the application startup mode.
#[derive(Debug)]
pub enum StartMode {
    /// Run the application as a server only. when running web server only,
    /// workers job will not handle.
    ServerOnly,
    /// Run the application web server and the worker in the same process.
    ServerAndWorker,
    /// Pulling job worker and execute them
    WorkerOnly,
    /// Run the app with all available components in the same process.
    All,
}

pub struct BootResult {
    /// Application Context
    pub app_context: AppContext,
    /// Web server routes
    pub router: Option<Router>,
    /// worker processor
    pub run_worker: bool,
    /// scheduler processor
    pub run_scheduler: bool,
}

/// Configuration structure for serving an application.
#[derive(Debug)]
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
pub async fn start<H: Hooks>(
    boot: BootResult,
    server_config: ServeParams,
    no_banner: bool,
) -> Result<()> {
    if boot.run_scheduler {
        let scheduler = scheduler::<H>(&boot.app_context, None, None, None)?;
        tokio::spawn(async move {
            let res = scheduler.run().await;
            if res.is_err() {
                error!(
                    err = res.unwrap_err().to_string(),
                    "error while running scheduler"
                );
            }
        });
    }

    if !no_banner {
        print_banner(&boot, &server_config);
    }

    let BootResult {
        router,
        run_worker,
        run_scheduler: _,
        app_context,
    } = boot;

    match (router, run_worker) {
        (Some(router), false) => {
            H::serve(router, &app_context, &server_config).await?;
        }
        (Some(router), true) => {
            let handle = if app_context.config.workers.mode == WorkerMode::BackgroundQueue {
                Some(start_queue_worker(&app_context)?)
            } else {
                None
            };

            H::serve(router, &app_context, &server_config).await?;

            if let Some(handle) = handle {
                shutdown_and_await_queue_worker(&app_context, handle).await?;
            }
        }
        (None, true) => {
            let handle = if app_context.config.workers.mode == WorkerMode::BackgroundQueue {
                Some(start_queue_worker(&app_context)?)
            } else {
                None
            };

            shutdown_signal().await;

            if let Some(handle) = handle {
                shutdown_and_await_queue_worker(&app_context, handle).await?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn start_queue_worker(app_context: &AppContext) -> Result<JoinHandle<()>> {
    debug!("note: worker is run in-process (tokio spawn)");

    if let Some(queue) = &app_context.queue_provider {
        let cloned_queue = queue.clone();
        let handle = tokio::spawn(async move {
            let res = cloned_queue.run().await;
            if res.is_err() {
                error!(
                    err = res.unwrap_err().to_string(),
                    "error while running worker"
                );
            }
        });
        return Ok(handle);
    }

    Err(Error::QueueProviderMissing)
}

async fn shutdown_and_await_queue_worker(
    app_context: &AppContext,
    handle: JoinHandle<()>,
) -> Result<(), Error> {
    if let Some(queue) = &app_context.queue_provider {
        queue.shutdown()?;
    }

    println!("press ctrl-c again to force quit");
    select! {
        _ = handle => {}
        () = shutdown_signal() => {}
    }
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

/// Initializes a new scheduler instance based on the provided configuration and context.
fn scheduler<H: Hooks>(
    app_context: &AppContext,
    config: Option<&PathBuf>,
    name: Option<String>,
    tag: Option<String>,
) -> Result<Scheduler> {
    let env_config_path = env::var(env_vars::SCHEDULER_CONFIG).ok();

    let config_path: Option<&Path> = config.map_or_else(
        || env_config_path.as_deref().map(Path::new),
        |path| Some(path.as_path()),
    );

    let scheduler = match config_path {
        Some(path) => Scheduler::from_config::<H>(path, &app_context.environment)?,
        None => {
            if let Some(config) = &app_context.config.scheduler {
                Scheduler::new::<H>(config, &app_context.environment)?
            } else {
                return Err(Error::Scheduler(scheduler::Error::Empty));
            }
        }
    };

    Ok(scheduler.by_spec(&scheduler::Spec { name, tag }))
}

/// Runs the scheduler with the given configuration and context. in case if list
/// args is true prints scheduler job configuration
///
/// This function initializes the scheduler, registers tasks through the
/// provided [`Hooks`], and executes the scheduler based on the specified
/// configuration or context. The scheduler continuously runs, managing and
/// executing scheduled tasks until a signal is received to shut down.
/// Upon receiving this signal, the function gracefully shuts down all running
/// tasks and exits safely.
///
/// # Errors
///
/// When running could not run the scheduler.
pub async fn run_scheduler<H: Hooks>(
    app_context: &AppContext,
    config: Option<&PathBuf>,
    name: Option<String>,
    tag: Option<String>,
    list: bool,
) -> Result<()> {
    let task_span = tracing::span!(tracing::Level::DEBUG, "scheduler_jobs");
    let _guard = task_span.enter();

    let scheduler = scheduler::<H>(app_context, config, name, tag)?;
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
    /// Seed database.
    Seed {
        reset: bool,
        from: PathBuf,
        dump: bool,
        dump_tables: Option<Vec<String>>,
    },
    /// Dump database schema
    Schema,
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
            db::migrate::<M>(&app_context.db).await?;
        }
        RunDbCommand::Down(steps) => {
            tracing::warn!("down:");
            db::down::<M>(&app_context.db, steps).await?;
        }
        RunDbCommand::Reset => {
            tracing::warn!("reset:");
            db::reset::<M>(&app_context.db).await?;
        }
        RunDbCommand::Status => {
            tracing::warn!("status:");
            db::status::<M>(&app_context.db).await?;
        }
        RunDbCommand::Entities => {
            tracing::warn!("entities:");

            tracing::warn!("{}", db::entities::<M>(app_context).await?);
        }
        RunDbCommand::Truncate => {
            tracing::warn!("truncate:");
            H::truncate(app_context).await?;
        }
        RunDbCommand::Seed {
            reset,
            from,
            dump,
            dump_tables,
        } => {
            tracing::warn!(reset = reset, from = %from.display(), "seed:");

            if dump || dump_tables.is_some() {
                db::dump_tables(&app_context.db, from.as_path(), dump_tables).await?;
            } else {
                if reset {
                    db::reset::<M>(&app_context.db).await?;
                }
                db::run_app_seed::<H>(app_context, &from).await?;
            }
        }
        RunDbCommand::Schema => {
            db::dump_schema(app_context, "schema_dump.json").await?;
            println!("Database schema dumped to 'schema_dump.json'");
        }
    }
    Ok(())
}

/// Initializes the application context by loading configuration and
/// establishing connections.
///
/// # Errors
/// When has an error to create DB connection.
pub async fn create_context<H: Hooks>(
    environment: &Environment,
    config: Config,
) -> Result<AppContext> {
    if config.logger.pretty_backtrace {
        std::env::set_var("RUST_BACKTRACE", "1");
        warn!(
            "pretty backtraces are enabled (this is great for development but has a runtime cost \
             for production. disable with `logger.pretty_backtrace` in your config yaml)"
        );
    }
    #[cfg(feature = "with-db")]
    let db = db::connect(&config.database).await?;

    let mailer = if let Some(cfg) = config.mailer.as_ref() {
        create_mailer(cfg)?
    } else {
        None
    };

    let queue_provider = bgworker::create_queue_provider(&config).await?;
    let ctx = AppContext {
        environment: environment.clone(),
        #[cfg(feature = "with-db")]
        db,
        queue_provider,
        storage: Storage::single(storage::drivers::null::new()).into(),
        cache: cache::Cache::new(cache::drivers::null::new()).into(),
        config,
        mailer,
    };

    H::after_context(ctx).await
}

#[cfg(feature = "with-db")]
/// Creates an application based on the specified mode and environment.
///
/// # Errors
///
/// When could not create the application
pub async fn create_app<H: Hooks, M: MigratorTrait>(
    mode: StartMode,
    environment: &Environment,
    config: Config,
) -> Result<BootResult> {
    let app_context = create_context::<H>(environment, config).await?;
    db::converge::<H, M>(&app_context, &app_context.config.database).await?;

    if let (Some(queue), Some(config)) = (&app_context.queue_provider, &app_context.config.queue) {
        bgworker::converge(queue, config).await?;
    }

    run_app::<H>(&mode, app_context).await
}

#[cfg(not(feature = "with-db"))]
pub async fn create_app<H: Hooks>(
    mode: StartMode,
    environment: &Environment,
    config: Config,
) -> Result<BootResult> {
    let app_context = create_context::<H>(environment, config).await?;

    if let (Some(queue), Some(config)) = (&app_context.queue_provider, &app_context.config.queue) {
        bgworker::converge(queue, config).await?;
    }

    run_app::<H>(&mode, app_context).await
}

/// Run the application with the  given mode
/// # Errors
///
/// When could not create the application
pub async fn run_app<H: Hooks>(mode: &StartMode, app_context: AppContext) -> Result<BootResult> {
    H::before_run(&app_context).await?;
    let initializers = H::initializers(&app_context).await?;

    info!(
        initializers = ?initializers.iter().map(|init| init.name()).collect::<Vec<_>>().join(","),
        "initializers loaded"
    );

    for initializer in &initializers {
        initializer.before_run(&app_context).await?;
    }

    match mode {
        StartMode::ServerOnly => {
            let router = setup_routes::<H>(&app_context, &initializers).await?;
            Ok(BootResult {
                app_context,
                router: Some(router),
                run_worker: false,
                run_scheduler: false,
            })
        }
        StartMode::ServerAndWorker => {
            register_workers::<H>(&app_context).await?;
            let router = setup_routes::<H>(&app_context, &initializers).await?;
            Ok(BootResult {
                app_context,
                router: Some(router),
                run_worker: true,
                run_scheduler: false,
            })
        }
        StartMode::All => {
            register_workers::<H>(&app_context).await?;
            let router = setup_routes::<H>(&app_context, &initializers).await?;
            Ok(BootResult {
                app_context,
                router: Some(router),
                run_worker: true,
                run_scheduler: true,
            })
        }
        StartMode::WorkerOnly => {
            register_workers::<H>(&app_context).await?;
            Ok(BootResult {
                app_context,
                router: None,
                run_worker: true,
                run_scheduler: false,
            })
        }
    }
}

/// Sets up the application's routes based on the provided initializers and hooks.
async fn setup_routes<H: Hooks>(
    app_context: &AppContext,
    initializers: &[Box<dyn Initializer>],
) -> Result<Router> {
    let app = H::before_routes(app_context).await?;
    let app = H::routes(app_context).to_router::<H>(app_context.clone(), app)?;
    let mut router = H::after_routes(app, app_context).await?;

    for initializer in initializers {
        router = initializer.after_routes(router, app_context).await?;
    }

    Ok(router)
}

async fn register_workers<H: Hooks>(app_context: &AppContext) -> Result<()> {
    if app_context.config.workers.mode == WorkerMode::BackgroundQueue {
        if let Some(queue) = &app_context.queue_provider {
            queue.register(MailerWorker::build(app_context)).await?;
            H::connect_workers(app_context, queue).await?;
        } else {
            return Err(Error::QueueProviderMissing);
        }

        debug!("done registering workers and queues");
    }
    Ok(())
}

#[must_use]
pub fn list_endpoints<H: Hooks>(ctx: &AppContext) -> Vec<ListRoutes> {
    H::routes(ctx).collect()
}

/// Waits for a shutdown signal, either via Ctrl+C or termination signal.
///
/// # Panics
///
/// This function will panic if it fails to install the signal handlers for
/// Ctrl+C or the terminate signal on Unix-based systems.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

pub struct MiddlewareInfo {
    pub id: String,
    pub enabled: bool,
    pub detail: String,
}

#[must_use]
pub fn list_middlewares<H: Hooks>(ctx: &AppContext) -> Vec<MiddlewareInfo> {
    H::middlewares(ctx)
        .iter()
        .map(|m| MiddlewareInfo {
            id: m.name().to_string(),
            enabled: m.is_enabled(),
            detail: m.config().unwrap_or_default().to_string(),
        })
        .collect::<Vec<_>>()
}

/// Initializes an [`EmailSender`] based on the mailer configuration settings
/// ([`config::Mailer`]).
fn create_mailer(config: &config::Mailer) -> Result<Option<EmailSender>> {
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
