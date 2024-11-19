use std::{env, path::PathBuf};

use clap::{Parser, Subcommand};
use loco_cli::{
    generate::{self, AssetsOption, BackgroundOption, DBOption},
    git, prompt, CmdExit,
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(global = true, short, long, value_enum, default_value = "ERROR")]
    /// Verbosity level
    verbose: LevelFilter,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Loco app
    New {
        /// Local path to generate into
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// App name
        #[arg(short, long)]
        name: Option<String>,

        /// Starter template
        #[arg(short, long)]
        template: Option<String>,

        /// DB Provider
        #[arg(long)]
        db: Option<DBOption>,

        /// Background worker configuration
        #[arg(long)]
        bg: Option<BackgroundOption>,

        /// Assets serving configuration
        #[arg(long)]
        assets: Option<AssetsOption>,
    },
}
#[allow(clippy::unnecessary_wraps)]
fn main() -> eyre::Result<()> {
    println!("");
    println!("");
    println!("!!!!!");
    println!("!!!!! NOTE: `loco-cli` is now replaced with `loco` which is a much more powerful ");
    println!("!!!!! and flexible new app creator for Loco. To install the new CLI run:");
    println!("!!!!!");
    println!("!!!!! $ cargo uninstall loco-cli && cargo install loco");
    println!("!!!!!");
    println!("");
    println!("");
    println!("");
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(cli.verbose.into())
                .from_env_lossy(),
        )
        .init();

    let res = match cli.command {
        Commands::New {
            path,
            template,
            db,
            bg,
            assets,
            name,
        } => {
            if env::var("ALLOW_IN_GIT_REPO").is_err()
                && git::is_a_git_repo(path.as_path()).unwrap_or(false)
            {
                prompt::warn_if_in_git_repo()?;
            }

            let app = prompt::app_name(name)?;

            let args = generate::ArgsPlaceholder {
                lib_name: app.to_string(),
                db,
                bg,
                assets,
                template,
            };

            tracing::debug!(args = format!("{:?}", args), "generate template args");
            match git::clone_template(path.as_path(), &app, &args) {
                Ok((path, messages)) => CmdExit::ok_with_message(&format!(
                    "\n🚂 Loco app generated successfully in:\n{}\n\n{}",
                    dunce::canonicalize(&path).unwrap_or(path).display(),
                    messages
                        .iter()
                        .map(|m| format!("- {m}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )),
                Err(err) => CmdExit::error_with_message(&format!("{err}")),
            }
        }
    };

    res.exit();
    Ok(())
}
