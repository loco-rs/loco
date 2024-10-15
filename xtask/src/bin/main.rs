use cargo_metadata::{semver::Version, MetadataCommand, Package};
use clap::{
    ArgAction::{SetFalse, SetTrue},
    Parser, Subcommand,
};
use colored::Colorize;
use std::env;
use xtask::fuzzy_steps;

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
    /// Bump loco version in all dependencies places
    BumpVersion {
        #[arg(name = "VERSION")]
        new_version: Version,
        #[arg(short, long, action = SetFalse)]
        exclude_starters: bool,
    },
    Fuzzy {
        #[arg(global = true, short, long, default_value_t = 1)]
        times: u64,
        #[arg(short, long, value_parser = clap::value_parser!(u64))]
        seed: Option<u64>,
        #[command(subcommand)]
        command: FuzzyCommands,
    },
}

#[derive(Subcommand)]
enum FuzzyCommands {
    GenerateTemplate,
    Scaffold,
}

struct FuzzyResult {
    seed: u64,
    error: Option<crazy_train::Error>,
}
fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    let project_dir = env::current_dir()?;
    println!("running in: {project_dir:?}");

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
        Commands::BumpVersion {
            new_version,
            exclude_starters,
        } => {
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
                xtask::bump_version::BumpVersion {
                    base_dir: project_dir,
                    version: new_version,
                    bump_starters: exclude_starters,
                }
                .run()?;
            }
            xtask::CmdExit::ok()
        }
        Commands::Fuzzy {
            command,
            seed,
            times,
        } => {
            let mut results: Vec<FuzzyResult> = (1..=times)
                .map(|_| {
                    let randomizer = seed.map_or_else(crazy_train::Randomizer::default, |seed| {
                        crazy_train::Randomizer::with_seed(seed)
                    });
                    let seed = randomizer.seed;
                    let temp_dir = env::temp_dir().join("loco");

                    let runner = match command {
                        FuzzyCommands::GenerateTemplate => {
                            fuzzy_steps::generate_project::run(randomizer, temp_dir.as_path())
                        }
                        FuzzyCommands::Scaffold => {
                            fuzzy_steps::scaffold::run(randomizer, temp_dir.as_path())
                        }
                    };

                    let result: Result<(), crazy_train::Error> = runner.run();

                    if temp_dir.exists() {
                        std::fs::remove_dir_all(temp_dir).expect("remove dir");
                    }
                    FuzzyResult {
                        seed,
                        error: result.err(),
                    }
                })
                .collect();

            results.sort_by(|a, b| a.error.is_some().cmp(&b.error.is_some()));
            let mut has_error = false;

            println!();
            println!("====================================");
            println!("          Results Summary           ");
            println!("====================================");

            for result in results {
                if let Some(err) = result.error {
                    has_error = true;
                    println!(
                        "{}",
                        format!("seed {}: error\n\n {}\n", result.seed, err).red()
                    );
                } else {
                    println!("{}", format!("seed {}: passed", result.seed).green());
                }
            }

            if has_error {
                xtask::CmdExit::error_with_message("failed")
            } else {
                xtask::CmdExit::ok()
            }
        }
    };

    res.exit();
    Ok(())
}
