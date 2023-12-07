mod ci;
use clap::{Parser, Subcommand};
use std::env;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run test on all Loco resources
    Test {},
    /// Bump loco version in all dependencies places
    BumpVersion {
        #[arg(name = "VERSION")]
        v: String,
    },
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    let project_dir = env::current_dir()?.join("..");

    match cli.command {
        Commands::Test {} => {
            let res = ci::all_resources(project_dir.as_path());
            println!("{res:#?}");
        }
        Commands::BumpVersion { v } => {
            println!("TBD {v}");
        }
    }

    Ok(())
}
