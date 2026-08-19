use std::env;

use cargo_metadata::{semver::Version, MetadataCommand, Package};
use clap::{ArgAction::SetTrue, Parser, Subcommand};
use xtask::versions;

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
    Test {
        /// Test only Loco as a library
        #[arg(short, long, action = SetTrue)]
        quick: bool,
    },
    /// Bump every version a release touches. See `xtask::versions`.
    Bump {
        #[arg(name = "VERSION")]
        new_version: Version,
    },
    /// Parse every fenced `rust` block in the docs tree and fail on the ones
    /// that are not valid Rust. Syntax only — see `xtask::docs_syntax`.
    DocsSyntax,
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    let project_dir = env::current_dir()?;
    println!("running in: {}", project_dir.display());

    let res = match cli.command {
        Commands::Test { quick } => {
            let res = if quick {
                vec![xtask::ci::run(project_dir.as_path()).expect("test should have run")]
            } else {
                xtask::ci::all_resources(project_dir.as_path())?
            };
            println!("{}", xtask::out::print_ci_results(&res));
            xtask::CmdExit::ok()
        }
        Commands::Bump { new_version } => {
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
                versions::bump_version(&new_version)?;
            }
            xtask::CmdExit::ok()
        }
        Commands::DocsSyntax => {
            xtask::docs_syntax::run(&project_dir)?;
            xtask::CmdExit::ok_with_message("docs-syntax passed")
        }
    };

    res.exit();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn command_tree_is_well_formed() {
        Cli::command().debug_assert();
    }
}
