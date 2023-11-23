use std::{path::PathBuf, process::exit};

use clap::{Parser, Subcommand};
use loco_cli::{template::Starter, CmdExit};
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
        /// Local path to copy the template from.
        #[arg(name = "path", default_value = ".")]
        path: PathBuf,

        /// Folder name of folder template
        #[arg(short, long, default_value = "loco-site")]
        folder_name: String,

        /// Rust lib name in Cargo.toml.
        #[arg(short, long)]
        lib_name: Option<String>,

        /// Rust lib name in Cargo.toml.
        #[arg(short, long)]
        template: Option<Starter>,

        #[arg(hide = true, short, long)]
        branch: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        Commands::New {
            path,
            folder_name,
            lib_name,
            template,
            branch,
        } => {
            let selected_template =
                template.unwrap_or_else(|| Starter::prompt_selection().unwrap());

            let random_string: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(20)
                .map(char::from)
                .collect();

            let mut define = vec![format!("auth_secret={random_string}")];
            if let Some(lib_name) = lib_name {
                define.push(format!("lib_name={lib_name}"));
            }
            match loco_cli::generate::demo_site(
                &selected_template,
                &path,
                &folder_name,
                Some(define),
                branch,
            ) {
                Ok(path) => CmdExit::ok_with_message(&format!(
                    "\n💥 Loco website generated successfully in path: {}",
                    path.display()
                )),
                Err(err) => CmdExit::error_with_message(&format!("{err}")),
            }
        }
    };

    if let Some(message) = res.message {
        eprintln!("{message}");
    };

    exit(res.code);
}
