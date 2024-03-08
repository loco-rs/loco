use clap::{Parser, Subcommand};
use loco_cli::{generate, git, prompt, CmdExit};
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;

use tracing_subscriber::EnvFilter;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(global = true, short, long, action = clap::ArgAction::Count)]
    /// Verbosity level
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Loco website
    New {
        /// Local path to generate into
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
}
#[allow(clippy::unnecessary_wraps)]
fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(match cli.verbose {
                    // The default one for `from_default_env`
                    0 => LevelFilter::ERROR.into(),
                    1 => LevelFilter::WARN.into(),
                    2 => LevelFilter::INFO.into(),
                    3 => LevelFilter::DEBUG.into(),
                    _ => LevelFilter::TRACE.into(),
                })
                .from_env_lossy(),
        )
        .init();

    let res = match cli.command {
        Commands::New { path } => {
            if git::is_a_git_repo(&path).unwrap_or(false) {
                prompt::warn_if_in_git_repo()?;
            }

            let app = prompt::app_name()?;

            let args = generate::ArgsPlaceholder {
                lib_name: app.to_string(),
            };

            tracing::debug!(args = format!("{:?}", args), "generate template args");
            match git::clone_template(path.as_path(), &app, &args) {
                Ok(path) => CmdExit::ok_with_message(&format!(
                    "\n🚂 Loco app generated successfully in:\n{}",
                    path.display()
                )),
                Err(err) => CmdExit::error_with_message(&format!("{err}")),
            }
        }
    };

    res.exit();
    Ok(())
}
