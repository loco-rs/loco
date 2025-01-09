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
    } else {}
}

#[cfg(any(
    feature = "bg_redis",
    feature = "bg_pg",
    feature = "bg_sqlt",
    feature = "with-db"
))]
use std::process::exit;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use clap::{ArgAction, Parser, Subcommand};
use colored::Colorize;
use duct::cmd;
use loco_gen::{Component, DeploymentKind, ScaffoldKind};

#[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
use crate::bgworker::JobStatus;
use crate::{
    app::{AppContext, Hooks},
    boot::{
        create_app, create_context, list_endpoints, list_middlewares, run_scheduler, run_task,
        start, RunDbCommand, ServeParams, StartMode,
    },
    config::Config,
    environment::{resolve_from_env, Environment, DEFAULT_ENVIRONMENT},
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
        /// disable the banner display
        #[arg(short, long, action = ArgAction::SetTrue)]
        no_banner: bool,
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
        #[arg(short = 'c', long = "config", action)]
        show_config: bool,
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
    #[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
    /// Managing jobs queue.
    Jobs {
        #[command(subcommand)]
        command: JobsCommands,
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
        #[arg(short = 'c', long = "config", action)]
        config_path: Option<PathBuf>,
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
        #[arg(short, long, action)]
        production: bool,
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
    #[command(after_help = format!(
    "{}
  - Generate empty model:
      $ cargo loco g model posts

  - Generate model with fields:
      $ cargo loco g model posts title:string! content:text

  - Generate model with references:
      $ cargo loco g model movies long_title:string director:references award:references:prize_id
      # 'director:references' references the 'directors' table with 'director_id' on 'movies'
      # 'award:references:prize_id' references the 'awards' table with 'prize_id' on 'movies'
",
    "Examples:".bold().underline()
))]
    Model {
        /// Name of the thing to generate
        name: String,

        /// Is it a link table? Use this in many-to-many relations
        #[arg(short, long, action)]
        link: bool,

        /// Model fields, eg. title:string hits:int
        #[clap(value_parser = parse_key_val::<String,String>)]
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    /// Generates a new migration file
    #[command(after_help = format!("{}
  - Create a new table:
      $ cargo loco g migration CreatePosts title:string
      # Creates a migration to add a 'posts' table with a 'title' column of type string.

  - Add columns to an existing table:
      $ cargo loco g migration AddNameAndAgeToUsers name:string age:int
      # Adds 'name' (string) and 'age' (integer) columns to the 'users' table.

  - Remove columns from a table:
      $ cargo loco g migration RemoveNameAndAgeFromUsers name:string age:int
      # Removes 'name' and 'age' columns from the 'users' table.

  - Add a foreign key reference:
      $ cargo loco g migration AddUserRefToPosts user:references
      # Adds a reference to the 'users' table in the 'posts' table.

  - Create a join table:
      $ cargo loco g migration CreateJoinTableUsersAndGroups count:int
      # Creates a join table 'users_groups' with an additional 'count' column.

  - Create an empty migration:
      $ cargo loco g migration FixUsersTable
      # Creates a blank migration file for custom edits to the 'users' table.

After running the migration, follow these steps to complete the process:
  - Apply the migration:
    $ cargo loco db migrate
  - Generate the model entities:
    $ cargo loco db entities
", "Examples:".bold().underline()))]
    Migration {
        /// Name of the migration to generate
        name: String,
        /// Table fields, eg. title:string hits:int
        #[clap(value_parser = parse_key_val::<String,String>, )]
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    /// Generates a CRUD scaffold, model and controller
    #[command(after_help = format!("{}
 $ cargo loco g model posts title:string! user:references --api", "Examples:".bold().underline()))]
    Scaffold {
        /// Name of the thing to generate
        name: String,

        /// Model fields, eg. title:string hits:int
        #[clap(value_parser = parse_key_val::<String,String>)]
        fields: Vec<(String, String)>,

        /// The kind of scaffold to generate
        #[clap(short, long, value_enum, group = "scaffold_kind_group")]
        kind: Option<ScaffoldKind>,

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
    #[command(after_help = format!(
    "{}
  - Generate an empty controller:
      $ cargo loco generate controller posts --api

  - Generate a controller with actions:
      $ cargo loco generate controller posts --api list remove update
",
    "Examples:".bold().underline()
))]
    Controller {
        /// Name of the thing to generate
        name: String,

        /// Actions
        actions: Vec<String>,

        /// The kind of controller actions to generate
        #[clap(short, long, value_enum, group = "scaffold_kind_group")]
        kind: Option<ScaffoldKind>,

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
    Deployment {
        // deployment kind.
        #[clap(long, value_enum)]
        kind: DeploymentKind,
    },

    /// Override templates and allows you to take control of them. You can
    /// always go back when deleting the local template.
    #[command(after_help = format!("{}
  - Override a Specific File:
      * cargo loco generate override scaffold/api/controller.t
      * cargo loco generate override migration/add_columns.t

  - Override All Files in a Folder:
      * cargo loco generate override scaffold/htmx
      * cargo loco generate override task

  - Override All templates:
      * cargo loco generate override .
", "Examples:".bold().underline()))]
    Override {
        /// The path to a specific template or directory to copy.
        template_path: Option<String>,

        /// Show available templates to copy under the specified directory
        /// without actually coping them.
        #[arg(long, action)]
        info: bool,
    },
}

impl ComponentArg {
    fn into_gen_component(self, config: &Config) -> crate::Result<Component> {
        match self {
            #[cfg(feature = "with-db")]
            Self::Model { name, link, fields } => Ok(Component::Model { name, link, fields }),
            #[cfg(feature = "with-db")]
            Self::Migration { name, fields } => Ok(Component::Migration { name, fields }),
            #[cfg(feature = "with-db")]
            Self::Scaffold {
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

                Ok(Component::Scaffold { name, fields, kind })
            }
            Self::Controller {
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

                Ok(Component::Controller {
                    name,
                    actions,
                    kind,
                })
            }
            Self::Task { name } => Ok(Component::Task { name }),
            Self::Scheduler {} => Ok(Component::Scheduler {}),
            Self::Worker { name } => Ok(Component::Worker { name }),
            Self::Mailer { name } => Ok(Component::Mailer { name }),
            Self::Deployment { kind } => {
                let copy_asset_folder = &config
                    .server
                    .middlewares
                    .static_assets
                    .clone()
                    .map(|a| a.folder.path);

                let fallback_file = &config
                    .server
                    .middlewares
                    .static_assets
                    .clone()
                    .map(|a| a.fallback);

                Ok(Component::Deployment {
                    kind,
                    asset_folder: copy_asset_folder.clone(),
                    fallback_file: fallback_file.clone(),
                    host: config.server.host.clone(),
                    port: config.server.port,
                })
            }
            Self::Override {
                template_path: _,
                info: _,
            } => Err(crate::Error::string(
                "Error: Override could not be generated.",
            )),
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
    /// Seed your database with initial data or dump tables to files.
    Seed {
        /// Clears all data in the database before seeding.
        #[arg(short, long)]
        reset: bool,
        /// Dumps all database tables to files.
        #[arg(short, long)]
        dump: bool,
        /// Specifies specific tables to dump.
        #[arg(long, value_delimiter = ',')]
        dump_tables: Option<Vec<String>>,
        /// Specifies the folder containing seed files (defaults to
        /// 'src/fixtures').
        #[arg(long, default_value = "src/fixtures")]
        from: PathBuf,
    },
    /// Dump database schema
    Schema,
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
            DbCommands::Seed {
                reset,
                from,
                dump,
                dump_tables,
            } => Self::Seed {
                reset,
                from,
                dump,
                dump_tables,
            },
            DbCommands::Create => {
                unreachable!("Create db should't handled in the global db commands")
            }
            DbCommands::Schema => Self::Schema,
        }
    }
}

#[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
#[derive(Subcommand)]
enum JobsCommands {
    /// Cancels jobs with the specified names, setting their status to
    /// `cancelled`.
    Cancel {
        /// Names of jobs to cancel.
        #[arg(long)]
        name: String,
    },
    /// Deletes jobs that are either completed or cancelled.
    Tidy {},
    /// Deletes jobs based on their age in days.
    Purge {
        /// Deletes jobs with errors or cancelled, older than the specified
        /// maximum age in days.
        #[arg(long, default_value_t = 90)]
        max_age: i64,
        /// Limits the jobs being saved to those with specific criteria like
        /// completed or queued.
        #[arg(long, use_value_delimiter = true)]
        status: Option<Vec<JobStatus>>,
        /// Saves the details of jobs into a file before deleting them.
        #[arg(long)]
        dump: Option<PathBuf>,
    },
    /// Saves the details of all jobs to files in the specified folder.
    Dump {
        /// Limits the jobs being saved to those with specific criteria like
        /// completed or queued.
        #[arg(long, use_value_delimiter = true)]
        status: Option<Vec<JobStatus>>,
        /// Folder to save the job files (default: current directory).
        #[arg(short, long, default_value = ".")]
        folder: PathBuf,
    },
    /// Imports jobs from a file.
    Import {
        /// Path to the file containing job details to import.
        #[arg(short, long)]
        file: PathBuf,
    },
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

    let config = H::load_config(&environment).await?;

    if !H::init_logger(&config, &environment)? {
        logger::init::<H>(&config.logger)?;
    }

    let app_context = create_context::<H>(&environment, config).await?;
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

    let config = H::load_config(&environment).await?;

    if !H::init_logger(&config, &environment)? {
        logger::init::<H>(&config.logger)?;
    }

    let task_span = create_root_span(&environment);
    let _guard = task_span.enter();

    match cli.command {
        Commands::Start {
            worker,
            server_and_worker,
            binding,
            port,
            no_banner,
        } => {
            let start_mode = if worker {
                StartMode::WorkerOnly
            } else if server_and_worker {
                StartMode::ServerAndWorker
            } else {
                StartMode::ServerOnly
            };

            let boot_result = create_app::<H, M>(start_mode, &environment, config).await?;
            let serve_params = ServeParams {
                port: port.map_or(boot_result.app_context.config.server.port, |p| p),
                binding: binding
                    .unwrap_or_else(|| boot_result.app_context.config.server.binding.to_string()),
            };
            start::<H>(boot_result, serve_params, no_banner).await?;
        }
        #[cfg(feature = "with-db")]
        Commands::Db { command } => {
            if matches!(command, DbCommands::Create) {
                db::create(&config.database.uri).await?;
            } else {
                let app_context = create_context::<H>(&environment, config).await?;
                run_db::<H, M>(&app_context, command.into()).await?;
            }
        }
        #[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
        Commands::Jobs { command } => {
            handle_job_command::<H>(command, &environment, config).await?;
        }
        Commands::Routes {} => {
            let app_context = create_context::<H>(&environment, config).await?;
            show_list_endpoints::<H>(&app_context);
        }
        Commands::Middleware { show_config } => {
            let app_context = create_context::<H>(&environment, config).await?;
            let middlewares = list_middlewares::<H>(&app_context);
            for middleware in middlewares.iter().filter(|m| m.enabled) {
                println!(
                    "{:<22} {}",
                    middleware.id.bold(),
                    if show_config {
                        middleware.detail.as_str()
                    } else {
                        ""
                    }
                );
            }
            println!("\n");
            for middleware in middlewares.iter().filter(|m| !m.enabled) {
                println!("{:<22} (disabled)", middleware.id.bold().dimmed(),);
            }
        }
        Commands::Task { name, params } => {
            let vars = task::Vars::from_cli_args(params);
            let app_context = create_context::<H>(&environment, config).await?;
            run_task::<H>(&app_context, name.as_ref(), &vars).await?;
        }
        Commands::Scheduler {
            name,
            config_path,
            tag,
            list,
        } => {
            let app_context = create_context::<H>(&environment, config).await?;
            run_scheduler::<H>(&app_context, config_path.as_ref(), name, tag, list).await?;
        }
        Commands::Generate { component } => {
            handle_generate_command::<H>(component, &config)?;
        }
        Commands::Doctor {
            config: config_arg,
            production,
        } => {
            if config_arg {
                println!("{}", &config);
                println!("Environment: {}", &environment);
            } else {
                let mut should_exit = false;
                for (_, check) in doctor::run_all(&config, production).await? {
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

    let config = H::load_config(&environment).await?;

    if !H::init_logger(&config, &environment)? {
        logger::init::<H>(&config.logger)?;
    }

    let task_span = create_root_span(&environment);
    let _guard = task_span.enter();

    match cli.command {
        Commands::Start {
            worker,
            server_and_worker,
            binding,
            port,
            no_banner,
        } => {
            let start_mode = if worker {
                StartMode::WorkerOnly
            } else if server_and_worker {
                StartMode::ServerAndWorker
            } else {
                StartMode::ServerOnly
            };

            let boot_result = create_app::<H>(start_mode, &environment, config).await?;
            let serve_params = ServeParams {
                port: port.map_or(boot_result.app_context.config.server.port, |p| p),
                binding: binding.map_or(
                    boot_result.app_context.config.server.binding.to_string(),
                    |b| b,
                ),
            };
            start::<H>(boot_result, serve_params, no_banner).await?;
        }
        Commands::Routes {} => {
            let app_context = create_context::<H>(&environment, config).await?;
            show_list_endpoints::<H>(&app_context)
        }
        Commands::Middleware { show_config } => {
            let app_context = create_context::<H>(&environment, config).await?;
            let middlewares = list_middlewares::<H>(&app_context);
            for middleware in middlewares.iter().filter(|m| m.enabled) {
                println!(
                    "{:<22} {}",
                    middleware.id.bold(),
                    if show_config {
                        middleware.detail.as_str()
                    } else {
                        ""
                    }
                );
            }
            println!("\n");
            for middleware in middlewares.iter().filter(|m| !m.enabled) {
                println!("{:<22} (disabled)", middleware.id.bold().dimmed(),);
            }
        }
        Commands::Task { name, params } => {
            let vars = task::Vars::from_cli_args(params);
            let app_context = create_context::<H>(&environment, config).await?;
            run_task::<H>(&app_context, name.as_ref(), &vars).await?;
        }
        #[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
        Commands::Jobs { command } => {
            handle_job_command::<H>(command, &environment, config).await?
        }
        Commands::Scheduler {
            name,
            config_path,
            tag,
            list,
        } => {
            let app_context = create_context::<H>(&environment, config).await?;
            run_scheduler::<H>(&app_context, config_path.as_ref(), name, tag, list).await?;
        }
        Commands::Generate { component } => {
            handle_generate_command::<H>(component, &config)?;
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

    // Sort first by path, then ensure HTTP methods are in a consistent order
    routes.sort_by(|a, b| {
        let method_priority = |actions: &[_]| match actions
            .first()
            .map(ToString::to_string)
            .unwrap_or_default()
            .as_str()
        {
            "GET" => 0,
            "POST" => 1,
            "PUT" => 2,
            "PATCH" => 3,
            "DELETE" => 4,
            _ => 5,
        };

        let a_priority = method_priority(&a.actions);
        let b_priority = method_priority(&b.actions);

        a.uri.cmp(&b.uri).then(a_priority.cmp(&b_priority))
    });

    // Group routes by their first path segment and full path
    let mut path_groups: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();

    for router in routes {
        let path = router.uri.trim_start_matches('/');
        let segments: Vec<&str> = path.split('/').collect();
        let root = (*segments.first().unwrap_or(&"")).to_string();

        let actions_str = router
            .actions
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");

        path_groups
            .entry(root)
            .or_default()
            .entry(router.uri.to_string())
            .or_default()
            .push(actions_str);
    }

    // Print tree structure
    for (root, paths) in path_groups {
        println!("/{}", root.bold());
        let paths_count = paths.len();
        let mut path_idx = 0;

        for (path, methods) in paths {
            path_idx += 1;
            let is_last_path = path_idx == paths_count;
            let is_group = methods.len() > 1;

            // Print first method
            let prefix = if is_last_path && !is_group {
                "  └─ "
            } else {
                "  ├─ "
            };
            let colored_method = color_method(&methods[0]);
            println!("{prefix}{colored_method}\t{path}");

            // Print additional methods in group
            if is_group {
                for (i, method) in methods[1..].iter().enumerate() {
                    let is_last_in_group = i == methods.len() - 2;
                    let group_prefix = if is_last_path && is_last_in_group {
                        "  └─ "
                    } else {
                        "  │  "
                    };
                    let colored_method = color_method(method);
                    println!("{group_prefix}{colored_method}\t{path}");
                }

                // Add spacing between groups if not the last path
                if !is_last_path {
                    println!("  │");
                }
            }
        }
    }
}

fn color_method(method: &str) -> String {
    match method {
        "GET" => method.green().to_string(),
        "POST" => method.blue().to_string(),
        "PUT" => method.yellow().to_string(),
        "PATCH" => method.magenta().to_string(),
        "DELETE" => method.red().to_string(),
        _ => method.to_string(),
    }
}

fn create_root_span(environment: &Environment) -> tracing::Span {
    tracing::span!(tracing::Level::DEBUG, "app", environment = %environment)
}

#[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
async fn handle_job_command<H: Hooks>(
    command: JobsCommands,
    environment: &Environment,
    config: Config,
) -> crate::Result<()> {
    let app_context = create_context::<H>(environment, config).await?;
    let queue = app_context.queue_provider.map_or_else(
        || {
            println!("queue not configured");
            exit(1);
        },
        |queue_provider| queue_provider,
    );

    match &command {
        JobsCommands::Cancel { name } => queue.cancel_jobs(name).await,
        JobsCommands::Tidy {} => {
            queue
                .clear_by_status(vec![JobStatus::Completed, JobStatus::Cancelled])
                .await
        }
        JobsCommands::Purge {
            max_age,
            status,
            dump,
        } => {
            let status = status.as_ref().map_or_else(
                || {
                    vec![
                        JobStatus::Failed,
                        JobStatus::Cancelled,
                        JobStatus::Queued,
                        JobStatus::Completed,
                    ]
                },
                std::clone::Clone::clone,
            );

            if let Some(path) = dump {
                let dump_path = queue
                    .dump(path.as_path(), Some(&status), Some(*max_age))
                    .await?;

                println!("Jobs successfully dumped to: {}", dump_path.display());
            }

            queue.clear_jobs_older_than(*max_age, &status).await
        }
        JobsCommands::Dump { status, folder } => {
            let dump_path = queue.dump(folder.as_path(), status.as_ref(), None).await?;
            println!("Jobs successfully dumped to: {}", dump_path.display());
            Ok(())
        }
        JobsCommands::Import { file } => queue.import(file.as_path()).await,
    }
}

fn handle_generate_command<H: Hooks>(
    component: ComponentArg,
    config: &Config,
) -> crate::Result<()> {
    if let ComponentArg::Override {
        template_path,
        info,
    } = component
    {
        match (template_path, info) {
            // If no template path is provided, display the available templates,
            // ignoring the `--info` flag.
            (None, true | false) => {
                let templates = loco_gen::template::collect();
                println!("{}", format_templates_as_tree(templates));
            }
            // If a template path is provided and `--info` is enabled,
            // display the templates from the specified path.
            (Some(path), true) => {
                let templates = loco_gen::template::collect_files_path(Path::new(&path)).unwrap();
                println!("{}", format_templates_as_tree(templates));
            }
            // If a template path is provided and `--info` is disabled,
            // copy the template to the default local template path.
            (Some(path), false) => {
                let copied_files = loco_gen::copy_template(
                    Path::new(&path),
                    Path::new(loco_gen::template::DEFAULT_LOCAL_TEMPLATE),
                )?;
                if copied_files.is_empty() {
                    println!("{}", "No templates were found to copy.".red());
                } else {
                    println!(
                        "{}",
                        "The following templates were successfully copied:".green()
                    );
                    for f in copied_files {
                        println!(" * {}", f.display());
                    }
                }
            }
        }
    } else {
        let get_result = loco_gen::generate(
            &loco_gen::RRgen::default(),
            component.into_gen_component(config)?,
            &loco_gen::AppInfo {
                app_name: H::app_name().to_string(),
            },
        )?;
        let messages = loco_gen::collect_messages(&get_result);
        println!("{messages}");
    };
    Ok(())
}

#[must_use]
pub fn format_templates_as_tree(paths: Vec<PathBuf>) -> String {
    let mut categories: BTreeMap<String, BTreeMap<String, Vec<PathBuf>>> = BTreeMap::new();

    for path in paths {
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            let mut components = parent_str.split('/');
            if let Some(top_level) = components.next() {
                let top_key = top_level.to_string();
                let sub_key = components.next().unwrap_or("").to_string();

                categories
                    .entry(top_key)
                    .or_default()
                    .entry(sub_key)
                    .or_default()
                    .push(path);
            }
        }
    }

    let mut output = String::new();
    output.push_str("Available templates and directories to copy:\n\n");

    for (top_level, sub_categories) in &categories {
        output.push_str(&format!("{}", format!("{top_level}\n").yellow()));

        for (sub_category, paths) in sub_categories {
            if !sub_category.is_empty() {
                output.push_str(&format!("{}", format!(" └── {sub_category}\n").yellow()));
            }

            for path in paths {
                output.push_str(&format!(
                    "   └── {}\n",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ));
            }
        }
    }

    output.push_str(&format!("\n\n{}\n\n", "Usage Examples:".bold().green()));
    output.push_str(&format!("{}", "Override a Specific File:\n".bold()));
    output.push_str(&format!(
        " * cargo loco generate override {}\n",
        "scaffold/api/controller.t".yellow()
    ));
    output.push_str(&format!(
        " * cargo loco generate override {}",
        "migration/add_columns.t".yellow()
    ));
    output.push_str(&format!(
        "{}",
        "\n\nOverride All Files in a Folder:\n".bold()
    ));
    output.push_str(&format!(
        " * cargo loco generate override {}\n",
        "scaffold/htmx".yellow()
    ));
    output.push_str(&format!(
        " * cargo loco generate override {}",
        "task".yellow()
    ));
    // output.push_str(" * cargo loco generate override task");
    output.push_str(&format!("{}", "\n\nOverride All templates:\n".bold()));
    output.push_str(&format!(
        " * cargo loco generate override {}\n",
        ".".yellow()
    ));

    output
}
