use cargo_metadata::{semver::Version, MetadataCommand, Package};
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
        new_version: Version,
    },
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    let project_dir = env::current_dir()?.join("..");

    let res = match cli.command {
        Commands::Test {} => {
            let res = xtask::ci::all_resources(project_dir.as_path())?;
            xtask::out::ci_results(&res);
            xtask::CmdExit::ok()
        }
        Commands::BumpVersion { new_version } => {
            let meta = MetadataCommand::new()
                .manifest_path("./Cargo.toml")
                .current_dir(&project_dir)
                .exec()
                .unwrap();
            let root: &Package = meta.root_package().unwrap();
            if xtask::prompt::confirmation(&format!(
                "upgrading loco version from {} to {}",
                root.version, new_version,
            ))? {
                xtask::bump_version::bump(project_dir.as_path(), &new_version)?;
            }
            xtask::CmdExit::ok()
        }
    };

    res.exit();
    Ok(())
}
