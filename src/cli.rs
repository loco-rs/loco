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
use {crate::boot::run_db, crate::db, sea_orm_migration::MigratorTrait};

mod tree;

pub use tree::format_templates_as_tree;
use tree::show_list_endpoints;

use clap::{ArgAction, ArgGroup, Parser, Subcommand, ValueHint};
use colored::Colorize;
use duct::cmd;
use std::fmt::Write;
use std::path::PathBuf;
use std::process::exit;

#[cfg(feature = "worker")]
use crate::bgworker::JobStatus;
#[cfg(debug_assertions)]
use crate::controller;
use crate::{
    app::{AppContext, Hooks},
    boot::{
        create_app, create_context, list_middlewares, run_scheduler, run_task, start, RunDbCommand,
        ServeParams, StartMode,
    },
    config::Config,
    doctor,
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
        /// Run the scheduler
        #[arg(long, action, conflicts_with = "all")]
        scheduler: bool,
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
    #[cfg(feature = "worker")]
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
    /// Validate and diagnose configurations.
    Doctor {
        /// print out the current configurations.
        #[arg(short, long, action)]
        config: bool,
        /// Deprecated alias for `--environment production`.
        ///
        /// Checks the production environment, skipping the ones that only make
        /// sense on a development machine.
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
        /// Run the scheduler
        #[arg(long, action)]
        scheduler: bool,
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

  - Generate model without timestamps:
      $ cargo loco g model posts title:string content:text --without-tz
",
    "Examples:".bold().underline()
))]
    Model {
        /// Name of the thing to generate
        name: String,

        /// Generate model without timestamps (`created_at`, `updated_at` columns)
        #[arg(long, action)]
        without_tz: bool,

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

  - Create migration without timestamps:
      $ cargo loco g migration CreatePosts title:string --without-tz
      # Creates a migration without timestamp columns

  - Create join table without timestamps:
      $ cargo loco g migration CreateJoinTableUsersAndGroups count:int --without-tz
      # Creates a join table without timestamp columns

After running the migration, follow these steps to complete the process:
  - Apply the migration:
    $ cargo loco db migrate
  - Generate the model entities:
    $ cargo loco db entities
", "Examples:".bold().underline()))]
    Migration {
        /// Name of the migration to generate
        name: String,

        /// Generate migration without timestamps (`created_at`, `updated_at` columns)
        #[arg(long, action)]
        without_tz: bool,

        /// Table fields, eg. title:string hits:int
        #[clap(value_parser = parse_key_val::<String,String>, )]
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    /// Generates a CRUD scaffold, model and controller
    #[command(after_help = format!("{}
 $ cargo loco g model posts title:string! user:references

 $ cargo loco g scaffold posts title:string! user:references --without-tz

 $ cargo loco g scaffold posts title:string! --no-auth", "Examples:".bold().underline()))]
    Scaffold {
        /// Name of the thing to generate
        name: String,

        /// Generate scaffold without timestamps (`created_at`, `updated_at` columns)
        #[arg(long, action)]
        without_tz: bool,

        /// Generate public routes. Scaffolded handlers take an `auth::JWT`
        /// extractor by default, so they answer 401 without a bearer token.
        #[arg(long, action)]
        no_auth: bool,

        /// Deprecated in 1.0: generators are adaptive now. Accepted so existing
        /// commands keep working — `--api` is a no-op, `--html`/`--htmx` explain
        /// the React SPA move. Hidden to keep the new CLI surface clean.
        #[arg(long, action, hide = true)]
        api: bool,
        #[arg(long, action, hide = true)]
        html: bool,
        #[arg(long, action, hide = true)]
        htmx: bool,

        /// Model fields, eg. title:string hits:int
        #[clap(value_parser = parse_key_val::<String,String>)]
        fields: Vec<(String, String)>,
    },
    /// Generate a new controller with the given controller name, and test file.
    #[command(after_help = format!(
    "{}
  - Generate an empty controller:
      $ cargo loco generate controller posts

  - Generate a controller with actions:
      $ cargo loco generate controller posts list remove update

  - Generate a controller whose routes require a JWT:
      $ cargo loco generate controller posts --auth
",
    "Examples:".bold().underline()
))]
    Controller {
        /// Name of the thing to generate
        name: String,

        /// Add an `auth::JWT` extractor to every generated handler. A generated
        /// controller is public by default — the mirror of scaffold's
        /// `--no-auth`.
        #[arg(long, action)]
        auth: bool,

        /// Deprecated in 1.0: generators are adaptive now. Accepted so existing
        /// commands keep working — `--api` is a no-op, `--html`/`--htmx` explain
        /// the React SPA move. Hidden to keep the new CLI surface clean.
        #[arg(long, action, hide = true)]
        api: bool,
        #[arg(long, action, hide = true)]
        html: bool,
        #[arg(long, action, hide = true)]
        htmx: bool,

        /// Actions
        actions: Vec<String>,
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
        /// The type of deployment to generate
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
      * cargo loco generate override scaffold/api
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

/// Handle the scaffold/controller "kind" flags that 1.0's adaptive generators
/// removed (`--api` / `--html` / `--htmx`).
///
/// Scaffold now auto-detects headless vs. clientside from the app's `frontend/`,
/// and controllers are always API controllers — so no kind flag is needed. We
/// still *accept* the old flags (rather than letting clap reject them with a
/// cryptic `unexpected argument` error) so existing tutorials, blog posts, and
/// muscle memory keep working: `--api` is a no-op, `--html`/`--htmx` point at
/// the React SPA that replaced server-rendered views.
#[cfg(debug_assertions)]
// `html` and `htmx` are the flag names users typed; they cannot be renamed apart.
#[allow(clippy::similar_names)]
fn warn_legacy_scaffold_kind(api: bool, html: bool, htmx: bool) -> crate::Result<()> {
    if html || htmx {
        return Err(crate::Error::string(
            "`--html`/`--htmx` view scaffolds were replaced by the React SPA frontend in 1.0. \
             Generators are adaptive now: scaffold emits the React frontend automatically when \
             the app has a `frontend/`, and only the typed backend otherwise — no kind flag \
             needed. See https://loco.rs/docs/how-to/use-generators/",
        ));
    }
    if api {
        eprintln!(
            "note: `--api` is no longer needed — generators are adaptive in 1.0 (headless by \
             default, React frontend when the app has one)."
        );
    }
    Ok(())
}

#[cfg(debug_assertions)]
impl ComponentArg {
    fn into_gen_component(self, config: &Config) -> crate::Result<loco_gen::Component> {
        match self {
            #[cfg(feature = "with-db")]
            Self::Model {
                name,
                without_tz,
                fields,
            } => Ok(loco_gen::Component::Model {
                name,
                with_tz: !without_tz,
                fields,
            }),
            #[cfg(feature = "with-db")]
            Self::Migration {
                name,
                without_tz,
                fields,
            } => Ok(loco_gen::Component::Migration {
                name,
                with_tz: !without_tz,
                fields,
            }),
            #[cfg(feature = "with-db")]
            Self::Scaffold {
                name,
                without_tz,
                no_auth,
                api,
                html,
                htmx,
                fields,
            } => {
                warn_legacy_scaffold_kind(api, html, htmx)?;
                Ok(loco_gen::Component::Scaffold {
                    name,
                    with_tz: !without_tz,
                    fields,
                    // Adaptive: emit the React-SPA frontend only when this is a
                    // clientside app (its once-per-app `frontend/src/routes.tsx`
                    // exists). Headless/serverside apps get the typed backend only.
                    frontend: std::path::Path::new("frontend/src/routes.tsx").exists(),
                    auth: !no_auth,
                })
            }
            Self::Controller {
                name,
                auth,
                api,
                html,
                htmx,
                actions,
            } => {
                warn_legacy_scaffold_kind(api, html, htmx)?;
                Ok(loco_gen::Component::Controller {
                    name,
                    actions,
                    auth,
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
    Nginx,
    Lambda,
}

impl DeploymentKind {
    #[cfg(debug_assertions)]
    fn to_generator_component(&self, config: &Config) -> loco_gen::Component {
        let kind = match self {
            Self::Docker => {
                let is_client_side_rendering =
                    PathBuf::from("frontend").join("package.json").exists();

                loco_gen::DeploymentKind::Docker {
                    copy_paths: Self::runtime_asset_paths(config),
                    is_client_side_rendering,
                }
            }
            Self::Nginx => loco_gen::DeploymentKind::Nginx {
                host: config.server.host.clone(),
                port: config.server.port,
            },
            Self::Lambda => loco_gen::DeploymentKind::Lambda {
                db: cfg!(feature = "with-db"),
                include_paths: Self::runtime_asset_paths(config),
            },
        };
        loco_gen::Component::Deployment { kind }
    }

    /// Directories the app reads from disk at runtime (static assets / static
    /// file serving), so a deployment can carry them alongside the binary.
    /// Shared by the Docker generator (copies them into the image) and the
    /// Lambda generator (bundles them into the zip via cargo-lambda `include`).
    /// `config/` is not listed here — every deployment needs it, so the
    /// templates add it unconditionally.
    #[cfg(debug_assertions)]
    fn runtime_asset_paths(config: &Config) -> Vec<PathBuf> {
        let mut paths = vec![];
        if let Some(static_assets) = &config.server.middlewares.static_assets {
            let asset_folder = PathBuf::from(controller::views::engines::DEFAULT_ASSET_FOLDER);
            if asset_folder.exists() {
                paths.push(asset_folder.clone());
            }
            if !static_assets.folder.path.starts_with(&asset_folder) {
                paths.push(PathBuf::from(&static_assets.folder.path));
            }
            if !static_assets.fallback.starts_with(asset_folder) {
                paths.push(PathBuf::from(&static_assets.fallback));
            }
        }
        paths
    }
}

#[cfg(feature = "worker")]
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
    /// Moves failed jobs back to `queued` so they run again.
    ///
    /// Distinct from `requeue`, which only rescues jobs stranded in
    /// `processing` by a crashed worker and cannot touch a failed one.
    Retry {
        /// Retry only this job. Omit to retry every failed job.
        #[arg(long)]
        id: Option<String>,
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
        .ok_or_else(|| format!("expected `key:value`, found no `:` in `{s}`"))?;
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
pub async fn main<H: Hooks, M: MigratorTrait>() -> crate::Result<()> {
    let cli: Cli = Cli::parse();
    let environment: Environment = cli.environment.unwrap_or_else(resolve_from_env).into();

    // `doctor --production` used to be a filter over which checks ran, while the
    // config it checked stayed whatever the ambient environment resolved to —
    // `development` unless LOCO_ENV said otherwise. On a server that had not set
    // LOCO_ENV, it reported a clean bill of health for the development database
    // and never opened the production config at all. Resolve it here, before
    // anything is loaded, so the flag selects the environment it names.
    let environment = match cli.command {
        Commands::Doctor {
            production: true, ..
        } if environment != Environment::Production => {
            eprintln!(
                "`doctor --production` is deprecated; use `--environment production` (or set \
                 LOCO_ENV). Checking the production environment."
            );
            Environment::Production
        }
        _ => environment,
    };

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
            scheduler,
            binding,
            port,
            no_banner,
        } => {
            let start_mode = start_mode_from_flags(all, server_and_worker, worker, scheduler);

            let boot_result =
                create_app::<H, M>(start_mode, &environment, app_context.config).await?;
            let serve_params = ServeParams {
                port: port.unwrap_or(boot_result.app_context.config.server.port),
                binding: binding
                    .unwrap_or_else(|| boot_result.app_context.config.server.binding.clone()),
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
        command => dispatch_common::<H>(command, &environment, app_context).await?,
    }
    Ok(())
}

/// Handles every CLI command whose behavior does not depend on the `with-db`
/// feature (i.e. does not need the `M: MigratorTrait` generic), given an
/// already-initialized [`AppContext`]. Shared by both the `with-db` and
/// non-`with-db` flavors of [`main`].
#[allow(clippy::too_many_lines)]
#[allow(clippy::cognitive_complexity)]
async fn dispatch_common<H: Hooks>(
    command: Commands,
    environment: &Environment,
    app_context: AppContext,
) -> crate::Result<()> {
    match command {
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
                println!("{:<22} (disabled)", middleware.id.bold().dimmed());
            }
        }
        Commands::Task { name, params } => {
            let vars = task::Vars::from_cli_args(params);
            run_task::<H>(&app_context, name.as_ref(), &vars).await?;
        }
        #[cfg(feature = "worker")]
        Commands::Jobs { command } => {
            handle_job_command(command, &app_context).await?;
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
        Commands::Doctor {
            config: config_arg,
            production: _,
        } => {
            if config_arg {
                println!("{}", app_context.config);
                println!("Environment: {environment}");
            } else {
                // Which checks apply follows from the environment, not from a
                // separate flag that could disagree with it.
                let production = environment == &Environment::Production;

                let mut should_exit = false;
                for (_, check) in doctor::run_all::<H>(&app_context, production).await? {
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
            println!("{}", H::app_version());
        }
        Commands::Watch {
            worker,
            server_and_worker,
            scheduler,
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
            if scheduler {
                cmd_str.push_str(" --scheduler");
            }

            cmd("cargo-watch", &["-s", &cmd_str]).run().map_err(|err| {
                Error::Message(format!(
                    "failed to start with `cargo-watch`. Did you `cargo install \
                         cargo-watch`?. error details: `{err}`",
                ))
            })?;
        }
        // `Start` and (with `with-db`) `Db` are handled by the caller before
        // delegating here. Map them to an explicit error instead of a
        // wildcard arm so the match stays exhaustive against future
        // `Commands` variants.
        Commands::Start { .. } => {
            return Err(Error::string(
                "internal error: `Start` command must be handled by the caller, not \
                 `dispatch_common`",
            ));
        }
        #[cfg(feature = "with-db")]
        Commands::Db { .. } => {
            return Err(Error::string(
                "internal error: `Db` command must be handled by the caller, not \
                 `dispatch_common`",
            ));
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
            scheduler,
            binding,
            port,
            no_banner,
        } => {
            let start_mode = start_mode_from_flags(all, server_and_worker, worker, scheduler);

            let boot_result = create_app::<H>(start_mode, &environment, app_context.config).await?;
            let serve_params = ServeParams {
                port: port.map_or(boot_result.app_context.config.server.port, |p| p),
                binding: binding
                    .unwrap_or_else(|| boot_result.app_context.config.server.binding.clone()),
            };
            start::<H>(boot_result, serve_params, no_banner).await?;
        }
        command => dispatch_common::<H>(command, &environment, app_context).await?,
    }
    Ok(())
}

fn create_root_span(environment: &Environment) -> tracing::Span {
    tracing::span!(tracing::Level::DEBUG, "app", environment = %environment)
}

/// Resolves the [`StartMode`] from the `Commands::Start` CLI flags. Shared by
/// the `with-db` and non-`with-db` flavors of `main`.
fn start_mode_from_flags(
    all: bool,
    server_and_worker: bool,
    worker: Option<Vec<String>>,
    scheduler: bool,
) -> StartMode {
    if all || (server_and_worker && scheduler) {
        StartMode::All
    } else if server_and_worker {
        StartMode::ServerAndWorker
    } else if let Some(tags) = worker {
        if scheduler {
            StartMode::WorkerAndScheduler { tags }
        } else {
            StartMode::WorkerOnly { tags }
        }
    } else if scheduler {
        StartMode::ServerAndScheduler
    } else {
        StartMode::ServerOnly
    }
}

#[cfg(feature = "worker")]
async fn handle_job_command(command: JobsCommands, app_context: &AppContext) -> crate::Result<()> {
    let queue = app_context.queue_provider.clone().unwrap_or_else(|| {
        println!("queue not configured");
        exit(1);
    });

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
        JobsCommands::Retry { id } => {
            let retried = queue.retry_failed(id.as_deref()).await?;
            // The count is the whole point: `--id` on an already-retried or
            // non-existent job is not an error, it just matches nothing.
            println!("{retried} job(s) moved back to the queue");
            if retried > 0 {
                println!(
                    "note: on the Redis provider these are queued to `default` — the queue a job \
                     was submitted to is not recorded once it fails"
                );
            }
            Ok(())
        }
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
                // `new_generator()` builds an `RRgen` without a working
                // directory, so it writes relative to the cwd — the app root,
                // since that is where `cargo loco` runs.
                working_dir: ".".into(),
            },
        )?;
        let messages = loco_gen::collect_messages(&get_result);
        println!("{messages}");
        format_generated_code();
    }
    Ok(())
}

/// Best-effort `cargo fmt` over the app after a generator run.
///
/// Generated Rust comes out of Tera templates, and a template cannot know where
/// rustfmt would choose to break a line. `--no-auth` is the case that forced
/// this: dropping the `_auth: auth::JWT` argument shortens three of the five
/// scaffolded handler signatures enough that rustfmt wants them on one line —
/// and whether the fourth one fits depends on how long the resource name is, so
/// no fixed template can be canonical for every resource.
///
/// That matters because a generated app runs `cargo fmt --check` in its own CI
/// (`loco-new/base_template/.github/workflows/ci.yaml.t`): shipping
/// non-canonical output would fail a user's build on code they did not write.
/// `loco new` already formats what it generates, for the same reason.
///
/// Failure is ignored — rustfmt may not be installed, and a formatting miss is
/// no reason to fail a generation whose files are already on disk.
#[cfg(debug_assertions)]
fn format_generated_code() {
    let _ = std::process::Command::new("cargo")
        .arg("fmt")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};
    use rstest::rstest;

    /// Clap only validates the command tree when it is built, and nothing else
    /// in the suite builds the whole thing. Without this, a duplicated long
    /// flag or a conflicting short reaches users as a runtime panic on first
    /// run.
    #[test]
    fn command_tree_is_well_formed() {
        Cli::command().debug_assert();
        Playground::command().debug_assert();
    }

    #[rstest]
    #[case::default(false, false, None, false, StartMode::ServerOnly)]
    #[case::all(true, false, None, false, StartMode::All)]
    #[case::server_and_worker(false, true, None, false, StartMode::ServerAndWorker)]
    #[case::server_worker_scheduler(false, true, None, true, StartMode::All)]
    #[case::scheduler(false, false, None, true, StartMode::ServerAndScheduler)]
    #[case::worker(false, false, Some(vec![]), false, StartMode::WorkerOnly { tags: vec![] })]
    #[case::worker_and_scheduler(
        false, false, Some(vec!["mail".to_string()]), true,
        StartMode::WorkerAndScheduler { tags: vec!["mail".to_string()] }
    )]
    // `--all` outranks every other combination.
    #[case::all_wins(true, true, Some(vec![]), true, StartMode::All)]
    fn start_flags_pick_a_mode(
        #[case] all: bool,
        #[case] server_and_worker: bool,
        #[case] worker: Option<Vec<String>>,
        #[case] scheduler: bool,
        #[case] expected: StartMode,
    ) {
        assert_eq!(
            start_mode_from_flags(all, server_and_worker, worker, scheduler),
            expected
        );
    }

    #[test]
    fn key_value_pairs_split_on_the_first_colon() {
        assert_eq!(
            parse_key_val::<String, String>("url:http://example.com").unwrap(),
            ("url".to_string(), "http://example.com".to_string())
        );
        assert!(parse_key_val::<String, String>("no-separator")
            .unwrap_err()
            .to_string()
            .contains("key:value"));
    }

    #[test]
    fn legacy_api_flag_is_a_noop() {
        // `--api` is the headless default now — accepted, generation proceeds.
        assert!(warn_legacy_scaffold_kind(true, false, false).is_ok());
        assert!(warn_legacy_scaffold_kind(false, false, false).is_ok());
    }

    #[test]
    fn legacy_html_htmx_flags_error_with_guidance() {
        let html = warn_legacy_scaffold_kind(false, true, false)
            .unwrap_err()
            .to_string();
        assert!(html.contains("React SPA"), "got: {html}");
        let htmx = warn_legacy_scaffold_kind(false, false, true)
            .unwrap_err()
            .to_string();
        assert!(htmx.contains("React SPA"), "got: {htmx}");
    }

    // Regression for #1790: the 1.0 adaptive rebuild removed the scaffold/
    // controller kind flags, so `--api` (straight from the tutorials) failed
    // clap with `error: unexpected argument '--api' found`. They must still
    // PARSE so existing commands keep working.
    #[test]
    fn generate_controller_still_accepts_legacy_kind_flags() {
        for flag in ["--api", "--html", "--htmx"] {
            let parsed =
                Cli::try_parse_from(["loco", "generate", "controller", "notes", "list", flag]);
            assert!(
                parsed.is_ok(),
                "controller {flag} should parse, got: {:?}",
                parsed.err()
            );
        }
    }

    #[cfg(feature = "with-db")]
    #[test]
    fn generate_scaffold_still_accepts_legacy_kind_flags() {
        for flag in ["--api", "--html", "--htmx"] {
            let parsed = Cli::try_parse_from([
                "loco",
                "generate",
                "scaffold",
                "posts",
                "title:string",
                flag,
            ]);
            assert!(
                parsed.is_ok(),
                "scaffold {flag} should parse, got: {:?}",
                parsed.err()
            );
        }
    }
}
