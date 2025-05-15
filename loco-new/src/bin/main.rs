use std::{
    env,
    path::{Path, PathBuf},
    process::{exit, Command},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use duct::cmd;
use loco::{
    generator::{executer, extract_default_template, Generator},
    settings::Settings,
    wizard, Result, OS,
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(global = true, short, long, value_enum, default_value = "ERROR")]
    /// Verbosity level
    log: LevelFilter,

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

        /// DB Provider
        #[arg(long)]
        db: Option<wizard::DBOption>,

        /// Background worker configuration
        #[arg(long)]
        bg: Option<wizard::BackgroundOption>,

        /// Assets serving configuration
        #[arg(long)]
        assets: Option<wizard::AssetsOption>,

        /// Create the starter in target git repository
        #[arg(short, long)]
        allow_in_git_repo: bool,

        /// Create a Unix (linux, mac) or Windows optimized starter
        #[arg(long, default_value = DEFAULT_OS)]
        os: OS,
    },
}

#[cfg(unix)]
const DEFAULT_OS: &str = "linux";
#[cfg(not(unix))]
const DEFAULT_OS: &str = "windows";

#[allow(clippy::cognitive_complexity)]
fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(cli.log.into())
                .from_env_lossy(),
        )
        .init();

    let res = match cli.command {
        Commands::New {
            path,
            db,
            bg,
            assets,
            name,
            allow_in_git_repo,
            os,
        } => {
            tracing::debug!(path = ?path, db = ?db, bg=?bg, assets=?assets,name=?name, allow_in_git_repo=allow_in_git_repo, os=?os, "CLI options");
            if !allow_in_git_repo && is_a_git_repo(path.as_path()).unwrap_or(false) {
                tracing::debug!("the target directory is a Git repository");
                wizard::warn_if_in_git_repo()?;
            }

            let app_name = wizard::app_name(name)?;

            let to: PathBuf = path.canonicalize()?.join(&app_name);

            if to.exists() {
                CmdExit::error_with_message(format!(
                    "The specified path '{}' already exist",
                    to.display()
                ))
            } else {
                tracing::debug!(dir = %to.display(), "creating application directory");
                let temp_to = tree_fs::TreeBuilder::default().create()?;

                let args = wizard::ArgsPlaceholder { db, bg, assets };
                let user_selection = wizard::start(&args)?;

                let generator_tmp_folder = extract_default_template()?;
                tracing::debug!(
                    dir = %generator_tmp_folder.root.display(),
                    "temporary template folder created",

                );

                let executor = executer::FileSystem::new(
                    generator_tmp_folder.root.as_path(),
                    temp_to.root.as_path(),
                );

                let settings = Settings::from_wizard(&app_name, &user_selection, os);

                if let Ok(path) = env::var("LOCO_DEV_MODE_PATH") {
                    println!("⚠️ NOTICE: working in dev mode, pointing to local Loco on '{path}'");
                }

                let res = match Generator::new(Arc::new(executor), settings).run() {
                    Ok(()) => {
                        std::fs::create_dir_all(&to)?;
                        let copy_options = fs_extra::dir::CopyOptions::new().content_only(true);
                        fs_extra::dir::copy(&temp_to.root, &to, &copy_options)?;
                        tracing::debug!("loco template app generated successfully",);
                        if let Err(err) = cmd!("cargo", "fmt")
                            .dir(&to)
                            .stdout_null()
                            .stderr_null()
                            .run()
                        {
                            tracing::debug!(dir = %to.display(), err = %err,"failed to run 'cargo fmt'");
                        }

                        CmdExit::ok_with_message(format!(
                            "\n🚂 Loco app generated successfully in:\n{}\n\n{}",
                            to.display(),
                            user_selection
                                .message()
                                .iter()
                                .map(|m| format!("- {m}"))
                                .collect::<Vec<_>>()
                                .join("\n")
                        ))
                    }
                    Err(err) => {
                        tracing::error!(error = %err, args = format!("{args:?}"), "app generation failed due to template error.");
                        CmdExit::error_with_message("generate template failed")
                    }
                };

                if let Err(err) = std::fs::remove_dir_all(&generator_tmp_folder.root) {
                    tracing::warn!(
                        error = %err,
                        dir = %generator_tmp_folder.root.display(),
                        "failed to delete temporary generator folder"
                    );
                }
                res
            }
        }
    };

    res.exit();
    Ok(())
}

/// Check if a given path is a Git repository
///
/// # Errors
///
/// when git binary is not found or could not canonicalize the given path
pub fn is_a_git_repo(destination_path: &Path) -> Result<bool> {
    let destination_path = destination_path.canonicalize()?;
    match Command::new("git")
        .arg("-C")
        .arg(destination_path)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(err) => {
            tracing::debug!(error = err.to_string(), "git not found");
            Ok(false)
        }
    }
}

#[derive(Debug)]
pub struct CmdExit {
    pub code: i32,
    pub message: Option<String>,
}

impl CmdExit {
    #[must_use]
    pub fn error_with_message<S: Into<String>>(msg: S) -> Self {
        Self {
            code: 1,
            message: Some(format!("🙀 {}", msg.into())),
        }
    }

    #[must_use]
    pub fn ok_with_message<S: Into<String>>(msg: S) -> Self {
        Self {
            code: 0,
            message: Some(msg.into()),
        }
    }

    pub fn exit(&self) {
        if let Some(message) = &self.message {
            eprintln!("{message}");
        }

        exit(self.code);
    }
}
