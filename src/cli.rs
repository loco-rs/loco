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
#[cfg(feature = "with-db")]
use {crate::boot::run_db, crate::db, crate::doctor, sea_orm_migration::MigratorTrait};

use clap::{ArgAction, ArgGroup, Parser, Subcommand, ValueHint};
use colored::Colorize;
use duct::cmd;
use std::fmt::Write;
#[cfg(any(
    feature = "bg_redis",
    feature = "bg_pg",
    feature = "bg_sqlt",
    feature = "with-db"
))]
use std::process::exit;
use std::{collections::BTreeMap, path::PathBuf};

#[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
use crate::bgworker::JobStatus;
#[cfg(debug_assertions)]
use crate::controller;
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
    #[command(group(ArgGroup::new("start_mode").args(&["worker", "server_and_worker", "all"])))]
    #[clap(alias("s"))]
    Start {
        /// Start worker. Optionally provide tags to run specific jobs (e.g. --worker=tag1,tag2)
        #[arg(short, long, action, value_delimiter = ',', num_args = 0.., conflicts_with_all = &["server_and_worker", "all"])]
        worker: Option<Vec<String>>,
        /// Start the server and worker in the same process
        #[arg(short, long, action, conflicts_with_all = &["worker", "all"])]
        server_and_worker: bool,
        /// Start the server, worker, and scheduler in the same process
        #[arg(short, long, action, conflicts_with_all = &["worker", "server_and_worker"])]
        all: bool,
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
        #[arg(short = 'c', long = "config", action, value_hint = ValueHint::FilePath)]
        config_path: Option<PathBuf>,
        /// Show all configured jobs
        #[arg(short, long, action)]
        list: bool,
    },
    /// code generation creates a set of files and code templates based on a
    /// predefined set of rules.
    #[cfg(debug_assertions)]
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
        #[arg(short, long, action, value_delimiter = ',', num_args = 0..)]
        worker: Option<Vec<String>>,
        /// start same-process server and worker
        #[arg(short, long, action)]
        server_and_worker: bool,
    },
}

#[cfg(debug_assertions)]
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
        kind: Option<loco_gen::ScaffoldKind>,

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
        kind: Option<loco_gen::ScaffoldKind>,

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
    /// Generate data loader
    Data {
        /// Name of the thing to generate
        name: String,
    },
    /// Generate a deployment infrastructure
    Deployment {
        // deployment kind.
        #[clap(value_enum)]
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

#[cfg(debug_assertions)]
impl ComponentArg {
    fn into_gen_component(self, config: &Config) -> crate::Result<loco_gen::Component> {
        match self {
            #[cfg(feature = "with-db")]
            Self::Model { name, link, fields } => {
                Ok(loco_gen::Component::Model { name, link, fields })
            }
            #[cfg(feature = "with-db")]
            Self::Migration { name, fields } => Ok(loco_gen::Component::Migration { name, fields }),
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
                    loco_gen::ScaffoldKind::Htmx
                } else if html {
                    loco_gen::ScaffoldKind::Html
                } else if api {
                    loco_gen::ScaffoldKind::Api
                } else {
                    return Err(crate::Error::string(
                        "Error: One of `kind`, `htmx`, `html`, or `api` must be specified.",
                    ));
                };

                Ok(loco_gen::Component::Scaffold { name, fields, kind })
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
                    loco_gen::ScaffoldKind::Htmx
                } else if html {
                    loco_gen::ScaffoldKind::Html
                } else if api {
                    loco_gen::ScaffoldKind::Api
                } else {
                    return Err(crate::Error::string(
                        "Error: One of `kind`, `htmx`, `html`, or `api` must be specified.",
                    ));
                };

                Ok(loco_gen::Component::Controller {
                    name,
                    actions,
                    kind,
                })
            }
            Self::Task { name } => Ok(loco_gen::Component::Task { name }),
            Self::Scheduler {} => Ok(loco_gen::Component::Scheduler {}),
            Self::Worker { name } => Ok(loco_gen::Component::Worker { name }),
            Self::Mailer { name } => Ok(loco_gen::Component::Mailer { name }),
            Self::Data { name } => Ok(loco_gen::Component::Data { name }),
            Self::Deployment { kind } => Ok(kind.to_generator_component(config)),
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
    #[cfg(debug_assertions)]
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
            #[cfg(debug_assertions)]
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

#[derive(clap::ValueEnum, Clone)]
pub enum DeploymentKind {
    Docker,
    Shuttle,
    Nginx,
}

impl DeploymentKind {
    #[cfg(debug_assertions)]
    fn to_generator_component(&self, config: &Config) -> loco_gen::Component {
        let kind = match self {
            Self::Docker => {
                let mut copy_paths = vec![];

                if let Some(static_assets) = &config.server.middlewares.static_assets {
                    let asset_folder =
                        PathBuf::from(controller::views::engines::DEFAULT_ASSET_FOLDER);
                    if asset_folder.exists() {
                        copy_paths.push(asset_folder.clone());
                    }
                    if !static_assets.folder.path.starts_with(&asset_folder) {
                        copy_paths.push(PathBuf::from(&static_assets.folder.path));
                    }
                    if !static_assets.fallback.starts_with(asset_folder) {
                        copy_paths.push(PathBuf::from(&static_assets.fallback));
                    }
                }

                let is_client_side_rendering =
                    PathBuf::from("frontend").join("package.json").exists();

                loco_gen::DeploymentKind::Docker {
                    copy_paths,
                    is_client_side_rendering,
                }
            }
            Self::Shuttle => loco_gen::DeploymentKind::Shuttle {
                runttime_version: None,
            },
            Self::Nginx => loco_gen::DeploymentKind::Nginx {
                host: config.server.host.to_string(),
                port: config.server.port,
            },
        };
        loco_gen::Component::Deployment { kind }
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
    /// Change `processing` status to `queue`.
    Requeue {
        /// Change `processing` jobs older than the specified
        /// maximum age in minutes.
        #[arg(long, default_value_t = 0)]
        from_age: i64,
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
    let app_context = create_context::<H>(&environment, config).await?;

    if !H::init_logger(&app_context)? {
        logger::init::<H>(&app_context.config.logger)?;
    }

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
    let app_context = create_context::<H>(&environment, config).await?;

    if !H::init_logger(&app_context)? {
        logger::init::<H>(&app_context.config.logger)?;
    }

    let task_span = create_root_span(&environment);
    let _guard = task_span.enter();

    match cli.command {
        Commands::Start {
            worker,
            server_and_worker,
            all,
            binding,
            port,
            no_banner,
        } => {
            let start_mode = worker.map_or(
                if server_and_worker {
                    StartMode::ServerAndWorker
                } else if all {
                    StartMode::All
                } else {
                    StartMode::ServerOnly
                },
                |tags| StartMode::WorkerOnly { tags },
            );

            let boot_result =
                create_app::<H, M>(start_mode, &environment, app_context.config).await?;
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
                db::create(&app_context.config.database.uri).await?;
            } else {
                run_db::<H, M>(&app_context, command.into()).await?;
            }
        }
        #[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
        Commands::Jobs { command } => {
            handle_job_command::<H>(command, &environment, app_context.config).await?;
        }
        Commands::Routes {} => {
            let app_context = create_context::<H>(&environment, app_context.config).await?;
            show_list_endpoints::<H>(&app_context);
        }
        Commands::Middleware { show_config } => {
            let app_context = create_context::<H>(&environment, app_context.config).await?;
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
            let app_context = create_context::<H>(&environment, app_context.config).await?;
            run_task::<H>(&app_context, name.as_ref(), &vars).await?;
        }
        Commands::Scheduler {
            name,
            config_path,
            tag,
            list,
        } => {
            let app_context = create_context::<H>(&environment, app_context.config).await?;
            run_scheduler::<H>(&app_context, config_path.as_ref(), name, tag, list).await?;
        }
        #[cfg(debug_assertions)]
        Commands::Generate { component } => {
            handle_generate_command::<H>(component, &app_context.config)?;
        }
        Commands::Doctor {
            config: config_arg,
            production,
        } => {
            if config_arg {
                println!("{}", &app_context.config);
                println!("Environment: {}", &environment);
            } else {
                let mut should_exit = false;
                for (_, check) in doctor::run_all(&app_context.config, production).await? {
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
            let mut cmd_str = String::from("cargo loco start");

            if let Some(worker_tags) = worker {
                if worker_tags.is_empty() {
                    cmd_str.push_str(" --worker");
                } else {
                    write!(cmd_str, " --worker={}", worker_tags.join(","))
                        .expect("Failed to write to string");
                }
            } else if server_and_worker {
                cmd_str.push_str(" --server-and-worker");
            }

            cmd("cargo-watch", &["-s", &cmd_str]).run().map_err(|err| {
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
    let app_context = create_context::<H>(&environment, config).await?;

    if !H::init_logger(&app_context)? {
        logger::init::<H>(&app_context.config.logger)?;
    }

    let task_span = create_root_span(&environment);
    let _guard = task_span.enter();

    match cli.command {
        Commands::Start {
            worker,
            server_and_worker,
            all,
            binding,
            port,
            no_banner,
        } => {
            let start_mode = worker.map_or(
                if server_and_worker {
                    StartMode::ServerAndWorker
                } else if all {
                    StartMode::All
                } else {
                    StartMode::ServerOnly
                },
                |tags| StartMode::WorkerOnly { tags },
            );

            let boot_result = create_app::<H>(start_mode, &environment, app_context.config).await?;
            let serve_params = ServeParams {
                port: port.map_or(boot_result.app_context.config.server.port, |p| p),
                binding: binding.map_or(
                    boot_result.app_context.config.server.binding.to_string(),
                    |b| b,
                ),
            };
            start::<H>(boot_result, serve_params, no_banner).await?;
        }
        Commands::Routes {} => show_list_endpoints::<H>(&app_context),
        Commands::Middleware { show_config } => {
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
            run_scheduler::<H>(&app_context, config_path.as_ref(), name, tag, list).await?;
        }
        #[cfg(debug_assertions)]
        Commands::Generate { component } => {
            handle_generate_command::<H>(component, &app_context.config)?;
        }
        Commands::Version {} => {
            println!("{}", H::app_version(),);
        }
        Commands::Watch {
            worker,
            server_and_worker,
        } => {
            // cargo-watch  -s 'cargo loco start'
            let mut cmd_str = String::from("cargo loco start");

            if let Some(worker_tags) = worker {
                if worker_tags.is_empty() {
                    cmd_str.push_str(" --worker");
                } else {
                    write!(cmd_str, " --worker={}", worker_tags.join(","))
                        .expect("Failed to write to string");
                }
            } else if server_and_worker {
                cmd_str.push_str(" --server-and-worker");
            }

            cmd("cargo-watch", &["-s", &cmd_str]).run().map_err(|err| {
                Error::Message(format!(
                    "failed to start with `cargo-watch`. Did you `cargo install \
                         cargo-watch`?. error details: `{err}`",
                ))
            })?;
        }
    }
    Ok(())
}

// Define route node structure with enhanced methods
#[derive(Default)]
struct RouteNode {
    children: BTreeMap<String, RouteNode>,
    endpoints: Vec<(String, String)>,
}

impl RouteNode {
    fn is_leaf(&self) -> bool {
        self.endpoints.len() == 1 && self.children.is_empty()
    }

    fn is_collapsible(&self) -> bool {
        self.endpoints.is_empty()
            && self.children.len() == 1
            && self.children.values().next().is_some_and(Self::is_leaf)
    }

    fn method(&self) -> &str {
        self.endpoints
            .first()
            .map_or("", |(method, _)| method.as_str())
    }

    fn print(&self, prefix: &str, segment: &str, is_last: bool, is_root: bool, current_path: &str) {
        match (is_root, self.is_leaf(), self.is_collapsible()) {
            // Root level special cases
            (true, true, _) => {
                Self::print_with_format(
                    &format!("/{segment}"),
                    &color_method(self.method()),
                    &Self::build_path(&[current_path, segment]),
                );
            }
            (true, _, true) => {
                let Some((child_segment, child_node)) = self.children.iter().next() else {
                    return;
                };
                Self::print_with_format(
                    &format!("/{segment}/{child_segment}"),
                    &color_method(child_node.method()),
                    &Self::build_path(&[current_path, segment, child_segment]),
                );
            }

            // Non root level special cases
            (false, true, _) => {
                let prefix_str = Self::format_prefix(prefix, is_last, true);

                Self::print_with_format(
                    &format!("{prefix_str}{segment}"),
                    &color_method(self.method()),
                    &Self::build_path(&[current_path, segment]),
                );
            }
            (false, _, true) => {
                let prefix_str = Self::format_prefix(prefix, is_last, true);
                let Some((child_segment, child_node)) = self.children.iter().next() else {
                    return;
                };
                Self::print_with_format(
                    &format!("{prefix_str}{segment}/{child_segment}"),
                    &color_method(child_node.method()),
                    &Self::build_path(&[current_path, segment, child_segment]),
                );
            }

            // Standard branch node handling
            _ => {
                if is_root {
                    println!("/{segment}");
                } else if !segment.is_empty() {
                    println!("{}{}", Self::format_prefix(prefix, is_last, true), segment);
                }

                // Print endpoints and children
                let next_prefix = Self::format_next_prefix(prefix, is_last);
                self.print_endpoints(
                    &next_prefix,
                    self.children.is_empty(),
                    &Self::build_path(&[current_path, segment]),
                );
                self.print_children(&next_prefix, &Self::build_path(&[current_path, segment]));
            }
        }
    }

    fn print_endpoints(&self, prefix: &str, is_last_group: bool, current_path: &str) {
        for (i, (method, _)) in self.endpoints.iter().enumerate() {
            let is_last_entry = i == self.endpoints.len() - 1 && is_last_group;
            let marker = if is_last_entry { "└─" } else { "├─" };
            Self::print_with_format(
                &format!("{prefix}{marker}"),
                &color_method(method),
                current_path,
            );
        }
    }

    fn print_children(&self, prefix: &str, current_path: &str) {
        let children = self.children.iter().collect::<Vec<_>>();
        for (i, (child_segment, child_node)) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;

            if child_node.is_leaf() {
                let marker = if is_last_child { "└─" } else { "├─" };
                Self::print_with_format(
                    &format!("{prefix}{marker} /{child_segment}"),
                    &color_method(child_node.method()),
                    &Self::build_path(&[current_path, child_segment]),
                );
            } else {
                child_node.print(prefix, child_segment, is_last_child, false, current_path);
            }
        }
    }

    fn format_prefix(prefix: &str, is_last: bool, with_slash: bool) -> String {
        let marker = if is_last { "└─" } else { "├─" };
        if with_slash {
            format!("{prefix}{marker} /")
        } else {
            format!("{prefix}{marker} ")
        }
    }

    fn format_next_prefix(prefix: &str, is_last: bool) -> String {
        if is_last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        }
    }

    fn build_path(segments: &[&str]) -> String {
        segments.iter().fold(String::new(), |mut acc, &segment| {
            if !segment.is_empty() {
                acc.push('/');
                acc.push_str(segment);
            }
            acc.replace("//", "/")
        })
    }

    fn print_with_format(tree: &str, method: &str, full_path: &str) {
        println!("{:<50} {}", format!("{tree} {method}"), full_path);
    }
}

fn show_list_endpoints<H: Hooks>(ctx: &AppContext) {
    // Get and sort routes
    let mut routes = list_endpoints::<H>(ctx);
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
        a.uri
            .cmp(&b.uri)
            .then(method_priority(&a.actions).cmp(&method_priority(&b.actions)))
    });

    // Build route tree
    let mut route_tree = RouteNode::default();
    for router in routes {
        let path = router.uri.trim_start_matches('/');
        let segments: Vec<&str> = path.split('/').collect();
        if segments.is_empty() {
            continue;
        }

        // Insert the route into the tree
        let mut current_node = &mut route_tree;
        for segment in &segments {
            current_node = current_node
                .children
                .entry((*segment).to_string())
                .or_default();
        }

        // Store the endpoint at this node
        current_node.endpoints.push((
            router
                .actions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            router.uri.clone(),
        ));
    }

    // Print the route tree
    for (i, (segment, node)) in route_tree.children.iter().enumerate() {
        node.print("", segment, i == route_tree.children.len() - 1, true, "");
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
        JobsCommands::Requeue { from_age } => queue.requeue(from_age).await,
    }
}

#[cfg(debug_assertions)]
fn handle_generate_command<H: Hooks>(
    component: ComponentArg,
    config: &Config,
) -> crate::Result<()> {
    use std::path::Path;
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
            &loco_gen::new_generator(),
            component.into_gen_component(config)?,
            &loco_gen::AppInfo {
                app_name: H::app_name().to_string(),
            },
        )?;
        let messages = loco_gen::collect_messages(&get_result);
        println!("{messages}");
    }
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

    let mut output = "Available templates and directories to copy:".to_string();
    let _ = writeln!(output);
    let _ = writeln!(output);

    for (top_level, sub_categories) in &categories {
        let _ = writeln!(output, "{}", top_level.to_string().yellow());

        for (sub_category, paths) in sub_categories {
            if !sub_category.is_empty() {
                let _ = writeln!(output, "{}", format!(" └── {sub_category}").yellow());
            }

            for path in paths {
                let _ = writeln!(
                    output,
                    "   └── {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
        }
    }

    let _ = writeln!(output);
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", "Usage Examples:".bold().green());
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", "Override a Specific File:".bold());

    let _ = writeln!(
        output,
        " * cargo loco generate override {}",
        "scaffold/api/controller.t".yellow()
    );
    let _ = writeln!(
        output,
        " * cargo loco generate override {}",
        "migration/add_columns.t".yellow()
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", "Override All Files in a Folder:".bold());
    let _ = writeln!(
        output,
        " * cargo loco generate override {}",
        "scaffold/htmx".yellow()
    );

    let _ = writeln!(
        output,
        " * cargo loco generate override {}",
        "task".yellow()
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", "Override All templates:".bold());
    let _ = writeln!(output, " * cargo loco generate override {}", ".".yellow());

    output
}
