use std::path::PathBuf;

use clap::{Parser, Subcommand};
use loco_cli::{
    template::{prompt_app, prompt_selection},
    CmdExit,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
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

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    let res = match cli.command {
        Commands::New { path } => {
            let app = prompt_app()?;
            let starter_url = prompt_selection()?;
            let random_string: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(20)
                .map(char::from)
                .collect();

            match loco_cli::generate::new_project(&starter_url, &path, &app, &random_string) {
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
