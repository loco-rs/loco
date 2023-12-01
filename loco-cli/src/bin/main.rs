use clap::{Parser, Subcommand};
use loco_cli::{generate, git, prompt, CmdExit};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::path::PathBuf;

use tracing_subscriber::EnvFilter;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
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
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let res = match cli.command {
        Commands::New { path } => {
            let app = prompt::app_name()?;

            let random_string: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(20)
                .map(char::from)
                .collect();

            let args = generate::ArgsPlaceholder {
                lib_name: app.to_string(),
                secret: random_string,
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
