use std::{env, path::PathBuf};

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
    /// Regenerate the `loco` agent skill's API index from rustdoc JSON and
    /// mirror the skill into the `loco new` app template, so neither can drift
    /// from the crate. See `xtask::agent_skill`.
    AgentSkill {
        /// Fail if the committed skill is stale instead of rewriting it.
        #[arg(long, action = SetTrue)]
        check: bool,
    },
    /// Measure whether an agent writes idiomatic Loco. See `xtask::eval`.
    Eval {
        /// List the task corpus and exit.
        #[arg(long, action = SetTrue)]
        list: bool,
        /// Gate every reference solution. The corpus's own test suite — a
        /// reference that fails silently invalidates every score built on it.
        #[arg(long, action = SetTrue)]
        check_references: bool,
        /// Restrict to a single task id.
        #[arg(long)]
        task: Option<String>,
        /// Skip the model judge and report gates and cost only.
        #[arg(long, action = SetTrue)]
        no_judge: bool,
        /// Score a previous run's saved answers instead of generating again.
        /// Recovers a run whose judging failed after generation was paid for.
        #[arg(long, action = SetTrue)]
        rejudge: bool,
        /// Reuse any arm whose answers were already saved, and generate only
        /// the rest. Unlike `--rejudge` this does not require every arm to
        /// have been answered, so a run that died partway — out of disk, out
        /// of memory — resumes without paying for generation twice.
        #[arg(long, action = SetTrue)]
        resume: bool,
        /// Scratch directory for scaffolded apps and the shared target dir.
        #[arg(long, default_value = "target/eval")]
        work: PathBuf,
    },
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
        Commands::AgentSkill { check } => {
            xtask::agent_skill::run(&project_dir, check)?;
            xtask::CmdExit::ok_with_message("agent-skill ok")
        }
        Commands::Eval {
            list,
            check_references,
            task,
            no_judge,
            rejudge,
            resume,
            work,
        } => {
            let work = if work.is_absolute() {
                work
            } else {
                project_dir.join(work)
            };
            if list {
                xtask::eval::list(&project_dir)?;
            } else if check_references {
                xtask::eval::check_references(&project_dir, &work, task.as_deref())?;
            } else {
                xtask::eval::run(
                    &project_dir,
                    &work,
                    task.as_deref(),
                    !no_judge,
                    rejudge,
                    resume,
                )?;
            }
            xtask::CmdExit::ok_with_message("eval ok")
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
