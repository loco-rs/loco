//! command-line interface for running various tasks and commands
//! related to the application. It allows developers to interact with the
//! application via the command line.
//!
//! # Example
//!
//! ```rust,ignore
//! use myapp::app::App;
//! use loco_rs::cli;
//! use migration::Migrator;
//!
//! #[tokio::main]
//! async fn main() {
//!     cli::main::<App, Migrator>().await
//! }
//! ```
cfg_if::cfg_if! {
    if #[cfg(feature = "with-db")] {
        use sea_orm_migration::MigratorTrait;
        use crate::doctor;
        use crate::boot::{run_db};
        use crate::db;
        use std::process::exit;
    } else {}
}

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use duct::cmd;

use crate::{
    app::{AppContext, Hooks},
    boot::{
        create_app, create_context, list_endpoints, list_middlewares, run_scheduler, run_task,
        start, RunDbCommand, ServeParams, StartMode,
    },
    environment::{resolve_from_env, Environment, DEFAULT_ENVIRONMENT},
    gen::{self, Component, ScaffoldKind},
    logger, task, Error,
};
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Playground {
    /// Specify the environment
    #[arg(short, long, global = true, help = &format!("Specify the environment [default: {}]", DEFAULT_ENVIRONMENT))]
    environment: Option<String>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Specify the environment
    #[arg(short, long, global = true, help = &format!("Specify the environment [default: {}]", DEFAULT_ENVIRONMENT))]
    environment: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an app
    #[clap(alias("s"))]
    Start {
        /// start worker
        #[arg(short, long, action)]
        worker: bool,
        /// start same-process server and worker
        #[arg(short, long, action)]
        server_and_worker: bool,
        /// server bind address
        #[arg(short, long, action)]
        binding: Option<String>,
        /// server port address
        #[arg(short, long, action)]
        port: Option<i32>,
    },
    #[cfg(feature = "with-db")]
    /// Perform DB operations
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
    /// Describe all application endpoints
    Routes {},
    /// Describe all application middlewares
    Middleware {
        // print out the middleware configurations.
        #[arg(short, long, action)]
        config: bool,
    },
    /// Run a custom task
    #[clap(alias("t"))]
    Task {
        /// Task name (identifier)
        name: Option<String>,
        /// Task params (e.g. <`my_task`> foo:bar baz:qux)
        #[clap(value_parser = parse_key_val::<String,String>)]
        params: Vec<(String, String)>,
    },
    /// Run the scheduler
    Scheduler {
        /// Run a specific job by its name.
        #[arg(short, long, action)]
        name: Option<String>,
        /// Run jobs that are associated with a specific tag.
        #[arg(short, long, action)]
        tag: Option<String>,
        /// Specify a path to a dedicated scheduler configuration file. by
        /// default load schedulers job setting from environment config.
        #[clap(value_parser)]
        #[arg(short, long, action)]
        config: Option<PathBuf>,
        /// Show all configured jobs
        #[arg(short, long, action)]
        list: bool,
    },
    /// code generation creates a set of files and code templates based on a
    /// predefined set of rules.
    #[clap(alias("g"))]
    Generate {
        /// What to generate
        #[command(subcommand)]
        component: ComponentArg,
    },
    #[cfg(feature = "with-db")]
    /// Validate and diagnose configurations.
    Doctor {
        /// print out the current configurations.
        #[arg(short, long, action)]
        config: bool,
    },
    /// Display the app version
    Version {},

    /// Watch and restart the app
    #[clap(alias("w"))]
    Watch {
        /// start worker
        #[arg(short, long, action)]
        worker: bool,
        /// start same-process server and worker
        #[arg(short, long, action)]
        server_and_worker: bool,
    },
}

#[derive(Subcommand)]
enum ComponentArg {
    #[cfg(feature = "with-db")]
    /// Generates a new model file for defining the data structure of your
    /// application, and test file logic.
    Model {
        /// Name of the thing to generate
        name: String,

        /// Is it a link table? Use this in many-to-many relations
        #[arg(short, long, action)]
        link: bool,

        /// Generate migration code only. Don't run the migration automatically.
        #[arg(short, long, action)]
        migration_only: bool,

        /// Model fields, eg. title:string hits:int
        #[clap(value_parser = parse_key_val::<String,String>)]
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    /// Generates a new migration file
    Migration {
        /// Name of the migration to generate
        name: String,
    },
    #[cfg(feature = "with-db")]
    /// Generates a CRUD scaffold, model and controller
    Scaffold {
        /// Name of the thing to generate
        name: String,

        /// Model fields, eg. title:string hits:int
        #[clap(value_parser = parse_key_val::<String,String>)]
        fields: Vec<(String, String)>,

        /// The kind of scaffold to generate
        #[clap(short, long, value_enum, group = "scaffold_kind_group")]
        kind: Option<gen::ScaffoldKind>,

        /// Use HTMX scaffold
        #[clap(long, group = "scaffold_kind_group")]
        htmx: bool,

        /// Use HTML scaffold
        #[clap(long, group = "scaffold_kind_group")]
        html: bool,

        /// Use API scaffold
        #[clap(long, group = "scaffold_kind_group")]
        api: bool,
    },
    /// Generate a new controller with the given controller name, and test file.
    Controller {
        /// Name of the thing to generate
        name: String,

        /// Actions
        actions: Vec<String>,

        /// The kind of controller actions to generate
        #[clap(short, long, value_enum, group = "scaffold_kind_group")]
        kind: Option<gen::ScaffoldKind>,

        /// Use HTMX controller actions
        #[clap(long, group = "scaffold_kind_group")]
        htmx: bool,

        /// Use HTML controller actions
        #[clap(long, group = "scaffold_kind_group")]
        html: bool,

        /// Use API controller actions
        #[clap(long, group = "scaffold_kind_group")]
        api: bool,
    },
    /// Generate a Task based on the given name
    Task {
        /// Name of the thing to generate
        name: String,
    },
    /// Generate a scheduler jobs configuration template
    Scheduler {},
    /// Generate worker
    Worker {
        /// Name of the thing to generate
        name: String,
    },
    /// Generate mailer
    Mailer {
        /// Name of the thing to generate
        name: String,
    },
    /// Generate a deployment infrastructure
    Deployment {},
}

impl TryFrom<ComponentArg> for Component {
    type Error = crate::Error;
    fn try_from(value: ComponentArg) -> Result<Self, Self::Error> {
        match value {
            #[cfg(feature = "with-db")]
            ComponentArg::Model {
                name,
                link,
                migration_only,
                fields,
            } => Ok(Self::Model {
                name,
                link,
                migration_only,
                fields,
            }),
            #[cfg(feature = "with-db")]
            ComponentArg::Migration { name } => Ok(Self::Migration { name }),
            #[cfg(feature = "with-db")]
            ComponentArg::Scaffold {
                name,
                fields,
                kind,
                htmx,
                html,
                api,
            } => {
                let kind = if let Some(kind) = kind {
                    kind
                } else if htmx {
                    ScaffoldKind::Htmx
                } else if html {
                    ScaffoldKind::Html
                } else if api {
                    ScaffoldKind::Api
                } else {
                    return Err(crate::Error::string(
                        "Error: One of `kind`, `htmx`, `html`, or `api` must be specified.",
                    ));
                };

                Ok(Self::Scaffold { name, fields, kind })
            }
            ComponentArg::Controller {
                name,
                actions,
                kind,
                htmx,
                html,
                api,
            } => {
                let kind = if let Some(kind) = kind {
                    kind
                } else if htmx {
                    ScaffoldKind::Htmx
                } else if html {
                    ScaffoldKind::Html
                } else if api {
                    ScaffoldKind::Api
                } else {
                    return Err(crate::Error::string(
                        "Error: One of `kind`, `htmx`, `html`, or `api` must be specified.",
                    ));
                };

                Ok(Self::Controller {
                    name,
                    actions,
                    kind,
                })
            }
            ComponentArg::Task { name } => Ok(Self::Task { name }),
            ComponentArg::Scheduler {} => Ok(Self::Scheduler {}),
            ComponentArg::Worker { name } => Ok(Self::Worker { name }),
            ComponentArg::Mailer { name } => Ok(Self::Mailer { name }),
            ComponentArg::Deployment {} => Ok(Self::Deployment {}),
        }
    }
}

#[derive(Subcommand)]
enum DbCommands {
    /// Create schema
    Create,
    /// Migrate schema (up)
    Migrate,
    /// Run one down migration, or add a number to run multiple down migrations
    /// (i.e. `down 2`)
    Down {
        /// The number of migrations to rollback
        #[arg(default_value_t = 1)]
        steps: u32,
    },
    /// Drop all tables, then reapply all migrations
    Reset,
    /// Migration status
    Status,
    /// Generate entity .rs files from database schema
    Entities,
    /// Truncate data in tables (without dropping)
    Truncate,
}

impl From<DbCommands> for RunDbCommand {
    fn from(value: DbCommands) -> Self {
        match value {
            DbCommands::Migrate => Self::Migrate,
            DbCommands::Down { steps } => Self::Down(steps),
            DbCommands::Reset => Self::Reset,
            DbCommands::Status => Self::Status,
            DbCommands::Entities => Self::Entities,
            DbCommands::Truncate => Self::Truncate,
            DbCommands::Create => {
                unreachable!("Create db should't handled in the global db commands")
            }
        }
    }
}

/// Parse a single key-value pair
fn parse_key_val<T, U>(
    s: &str,
) -> std::result::Result<(T, U), Box<dyn std::error::Error + Send + Sync>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    let pos = s
        .find(':')
        .ok_or_else(|| format!("invalid KEY=value: no `:` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

#[cfg(feature = "with-db")]
/// run playgroup code
///
/// # Errors
///
/// When could not create app context
pub async fn playground<H: Hooks>() -> crate::Result<AppContext> {
    let cli = Playground::parse();
    let environment: Environment = cli.environment.unwrap_or_else(resolve_from_env).into();

    let app_context = create_context::<H>(&environment).await?;
    Ok(app_context)
}

/// # Main CLI Function
///
/// The `main` function is the entry point for the command-line interface (CLI)
/// of the application. It parses command-line arguments, interprets the
/// specified commands, and performs corresponding actions. This function is
/// generic over `H` and `M`, where `H` represents the application hooks and `M`
/// represents the migrator trait for handling database migrations.
///
/// # Errors
///
/// Returns an any error indicating success or failure during the CLI execution.
///
/// # Example
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::cli;
/// use migration::Migrator;
///
/// #[tokio::main]
/// async fn main()  {
///     cli::main::<App, Migrator>().await
/// }
/// ```
#[cfg(feature = "with-db")]
#[allow(clippy::too_many_lines)]
#[allow(clippy::cognitive_complexity)]
pub async fn main<H: Hooks, M: MigratorTrait>() -> crate::Result<()> {
    let cli: Cli = Cli::parse();
    let environment: Environment = cli.environment.unwrap_or_else(resolve_from_env).into();

    let config = environment.load()?;

    if !H::init_logger(&config, &environment)? {
        logger::init::<H>(&config.logger);
    }

    let task_span = create_root_span(&environment);
    let _guard = task_span.enter();

    match cli.command {
        Commands::Start {
            worker,
            server_and_worker,
            binding,
            port,
        } => {
            let start_mode = if worker {
                StartMode::WorkerOnly
            } else if server_and_worker {
                StartMode::ServerAndWorker
            } else {
                StartMode::ServerOnly
            };

            let boot_result = create_app::<H, M>(start_mode, &environment).await?;
            let serve_params = ServeParams {
                port: port.map_or(boot_result.app_context.config.server.port, |p| p),
                binding: binding
                    .unwrap_or_else(|| boot_result.app_context.config.server.binding.to_string()),
            };
            start::<H>(boot_result, serve_params).await?;
        }
        #[cfg(feature = "with-db")]
        Commands::Db { command } => {
            if matches!(command, DbCommands::Create) {
                db::create(&environment.load()?.database.uri).await?;
            } else {
                let app_context = create_context::<H>(&environment).await?;
                run_db::<H, M>(&app_context, command.into()).await?;
            }
        }
        Commands::Routes {} => {
            let app_context = create_context::<H>(&environment).await?;
            show_list_endpoints::<H>(&app_context);
        }
        Commands::Middleware { config } => {
            let app_context = create_context::<H>(&environment).await?;
            let middlewares = list_middlewares::<H>(&app_context, config);
            for middleware in middlewares {
                println!("{middleware}");
            }
        }
        Commands::Task { name, params } => {
            let vars = task::Vars::from_cli_args(params);
            let app_context = create_context::<H>(&environment).await?;
            run_task::<H>(&app_context, name.as_ref(), &vars).await?;
        }
        Commands::Scheduler {
            name,
            config,
            tag,
            list,
        } => {
            let app_context = create_context::<H>(&environment).await?;
            run_scheduler::<H>(&app_context, config.as_ref(), name, tag, list).await?;
        }
        Commands::Generate { component } => {
            gen::generate::<H>(component.try_into()?, &config)?;
        }
        Commands::Doctor { config: config_arg } => {
            if config_arg {
                println!("{}", &config);
                println!("Environment: {}", &environment);
            } else {
                let mut should_exit = false;
                for (_, check) in doctor::run_all(&config).await {
                    if !should_exit && !check.valid() {
                        should_exit = true;
                    }
                    println!("{check}");
                }
                if should_exit {
                    exit(1);
                }
            }
        }
        Commands::Version {} => {
            println!("{}", H::app_version(),);
        }

        Commands::Watch {
            worker,
            server_and_worker,
        } => {
            // cargo-watch  -s 'cargo loco start'
            let mut subcmd = vec!["cargo", "loco", "start"];
            if worker {
                subcmd.push("--worker");
            } else if server_and_worker {
                subcmd.push("--server-and-worker");
            }

            cmd("cargo-watch", &["-s", &subcmd.join(" ")])
                .run()
                .map_err(|err| {
                    Error::Message(format!(
                        "failed to start with `cargo-watch`. Did you `cargo install \
                         cargo-watch`?. error details: `{err}`",
                    ))
                })?;
        }
    }
    Ok(())
}

#[cfg(not(feature = "with-db"))]
pub async fn main<H: Hooks>() -> crate::Result<()> {
    let cli = Cli::parse();
    let environment: Environment = cli.environment.unwrap_or_else(resolve_from_env).into();

    let config = environment.load()?;

    if !H::init_logger(&config, &environment)? {
        logger::init::<H>(&config.logger);
    }

    let task_span = create_root_span(&environment);
    let _guard = task_span.enter();

    match cli.command {
        Commands::Start {
            worker,
            server_and_worker,
            binding,
            port,
        } => {
            let start_mode = if worker {
                StartMode::WorkerOnly
            } else if server_and_worker {
                StartMode::ServerAndWorker
            } else {
                StartMode::ServerOnly
            };

            let boot_result = create_app::<H>(start_mode, &environment).await?;
            let serve_params = ServeParams {
                port: port.map_or(boot_result.app_context.config.server.port, |p| p),
                binding: binding.map_or(
                    boot_result.app_context.config.server.binding.to_string(),
                    |b| b,
                ),
            };
            start::<H>(boot_result, serve_params).await?;
        }
        Commands::Routes {} => {
            let app_context = create_context::<H>(&environment).await?;
            show_list_endpoints::<H>(&app_context)
        }
        Commands::Middleware { config } => {
            let app_context = create_context::<H>(&environment).await?;
            let middlewares = list_middlewares::<H>(&app_context, config);
            for middleware in middlewares {
                println!("{middleware}");
            }
        }
        Commands::Task { name, params } => {
            let vars = task::Vars::from_cli_args(params);
            let app_context = create_context::<H>(&environment).await?;
            run_task::<H>(&app_context, name.as_ref(), &vars).await?;
        }
        Commands::Scheduler {
            name,
            config,
            tag,
            list,
        } => {
            let app_context = create_context::<H>(&environment).await?;
            run_scheduler::<H>(&app_context, config.as_ref(), name, tag, list).await?;
        }
        Commands::Generate { component } => {
            gen::generate::<H>(component.try_into()?, &config)?;
        }
        Commands::Version {} => {
            println!("{}", H::app_version(),);
        }
        Commands::Watch {
            worker,
            server_and_worker,
        } => {
            // cargo-watch  -s 'cargo loco start'
            let mut subcmd = vec!["cargo", "loco", "start"];
            if worker {
                subcmd.push("--worker");
            } else if server_and_worker {
                subcmd.push("--server-and-worker");
            }

            cmd("cargo-watch", &["-s", &subcmd.join(" ")])
                .run()
                .map_err(|err| {
                    Error::Message(format!(
                        "failed to start with `cargo-watch`. Did you `cargo install \
                         cargo-watch`?. error details: `{err}`",
                    ))
                })?;
        }
    }
    Ok(())
}

fn show_list_endpoints<H: Hooks>(ctx: &AppContext) {
    let mut routes = list_endpoints::<H>(ctx);
    routes.sort_by(|a, b| a.uri.cmp(&b.uri));
    for router in routes {
        println!("{router}");
    }
}

fn create_root_span(environment: &Environment) -> tracing::Span {
    tracing::span!(tracing::Level::DEBUG, "app", environment = %environment)
}
