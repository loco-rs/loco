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

use clap::{Parser, Subcommand};

use crate::{
    app::{AppContext, Hooks},
    boot::{
        create_app, create_context, list_endpoints, run_task, start, RunDbCommand, ServeParams,
        StartMode,
    },
    environment::{resolve_from_env, Environment, DEFAULT_ENVIRONMENT},
    gen::{self, Component},
    logger, task,
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
    /// Run a custom task
    Task {
        /// Task name (identifier)
        name: Option<String>,
        /// Task params (e.g. <`my_task`> foo:bar baz:qux)
        #[clap(value_parser = parse_key_val::<String,String>)]
        params: Vec<(String, String)>,
    },
    /// code generation creates a set of files and code templates based on a
    /// predefined set of rules.
    Generate {
        /// What to generate
        #[command(subcommand)]
        component: ComponentArg,
    },
    #[cfg(feature = "with-db")]
    /// Validate and diagnose configurations.
    Doctor {},
    /// Display the app version
    Version {},
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
        #[clap(short, long, value_enum, default_value_t = gen::ScaffoldKind::Api)]
        kind: gen::ScaffoldKind,
    },
    /// Generate a new controller with the given controller name, and test file.
    Controller {
        /// Name of the thing to generate
        name: String,
    },
    /// Generate a Task based on the given name
    Task {
        /// Name of the thing to generate
        name: String,
    },
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

impl From<ComponentArg> for Component {
    fn from(value: ComponentArg) -> Self {
        match value {
            #[cfg(feature = "with-db")]
            ComponentArg::Model {
                name,
                link,
                migration_only,
                fields,
            } => Self::Model {
                name,
                link,
                migration_only,
                fields,
            },
            #[cfg(feature = "with-db")]
            ComponentArg::Migration { name } => Self::Migration { name },
            #[cfg(feature = "with-db")]
            ComponentArg::Scaffold { name, fields, kind } => Self::Scaffold { name, fields, kind },
            ComponentArg::Controller { name } => Self::Controller { name },
            ComponentArg::Task { name } => Self::Task { name },
            ComponentArg::Worker { name } => Self::Worker { name },
            ComponentArg::Mailer { name } => Self::Mailer { name },
            ComponentArg::Deployment {} => Self::Deployment {},
        }
    }
}

#[derive(Subcommand)]
enum DbCommands {
    /// Create schema
    Create,
    /// Migrate schema (up)
    Migrate,
    /// Run one down migration, or add a number to run multiple down migrations (i.e. `down 2`)
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
pub async fn main<H: Hooks, M: MigratorTrait>() -> eyre::Result<()> {
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
        Commands::Task { name, params } => {
            let vars = task::Vars::from_cli_args(params);
            let app_context = create_context::<H>(&environment).await?;
            run_task::<H>(&app_context, name.as_ref(), &vars).await?;
        }
        Commands::Generate { component } => {
            gen::generate::<H>(component.into(), &config)?;
        }
        Commands::Doctor {} => {
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
        Commands::Version {} => {
            println!("{}", H::app_version(),);
        }
    }
    Ok(())
}

#[cfg(not(feature = "with-db"))]
pub async fn main<H: Hooks>() -> eyre::Result<()> {
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
        Commands::Task { name, params } => {
            let vars = task::Vars::from_cli_args(params);
            let app_context = create_context::<H>(&environment).await?;
            run_task::<H>(&app_context, name.as_ref(), &vars).await?;
        }
        Commands::Generate { component } => {
            gen::generate::<H>(component.into(), &config)?;
        }
        Commands::Version {} => {
            println!("{}", H::app_version(),);
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
