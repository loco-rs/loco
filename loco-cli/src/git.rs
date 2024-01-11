use crate::generate;
use crate::prompt;
use fs_extra::dir::{copy, CopyOptions};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// getting logo debug path for working locally.
///
/// # Errors
/// when the path is not given
pub fn debug_path() -> Option<PathBuf> {
    env::var("LOCO_DEBUG_PATH").ok().map(PathBuf::from)
}

const BASE_REPO_URL: &str = "https://github.com/loco-rs/loco.git";
const BRANCH_NAME: &str = "master"; // Branches are currently only supported with "git" natively installed

/// Define the starter template in Loco repository
const STARTER_TEMPLATE_FOLDER: &str = "starters";

/// Clone a Loco template to the specified destination folder.
///
/// This function takes a destination path, a folder name, and additional generation arguments.
/// It clones the Loco template, prompts the user to select a template, and generates the project
/// in the specified destination folder with the provided arguments.
///
/// # Errors
/// 1. when the `destination_path` is invalid
/// 2. could not collect templates files
/// 3. could not prompt template selection to the user
pub fn clone_template(
    destination_path: &Path,
    folder_name: &str,
    args: &generate::ArgsPlaceholder,
) -> eyre::Result<PathBuf> {
    let destination_path = destination_path.canonicalize()?;
    let copy_template_to = destination_path.join(folder_name);

    if copy_template_to.exists() {
        eyre::bail!(
            "The specified path '{}' already exist",
            copy_template_to.display()
        );
    }

    // in case of debug path is given, we skipping cloning project and working on the given directory
    let loco_project_path = match debug_path() {
        Some(p) => p,
        None => clone_repo()?,
    };

    tracing::debug!("loco project path: {:?}", loco_project_path);

    let starters_path = loco_project_path.join(STARTER_TEMPLATE_FOLDER);

    let templates = generate::collect_templates(&starters_path)?;

    let (folder, template) = prompt::template_selection(&templates)?;

    if !Path::new(&copy_template_to).exists() {
        std::fs::create_dir(&copy_template_to)?;
    }

    let copy_from = starters_path.join(folder);
    tracing::debug!(
        from = copy_from.display().to_string(),
        to = copy_template_to.display().to_string(),
        "copy starter project"
    );

    copy(
        &copy_from,
        &copy_template_to,
        &CopyOptions::default().content_only(true),
    )?;

    if debug_path().is_none() {
        tracing::debug!(
            folder = copy_from.display().to_string(),
            "deleting temp folder"
        );
        if let Err(e) = std::fs::remove_dir_all(&copy_from) {
            tracing::debug!(
                folder = copy_from.display().to_string(),
                error = e.to_string(),
                "deleting temp folder is failed"
            );
        }
    }

    template.generate(&copy_template_to, args);

    Ok(copy_template_to)
}

fn clone_repo() -> eyre::Result<PathBuf> {
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();

    let temp_clone_dir = env::temp_dir().join(random_string);
    
    let repo_url = env::var("STARTER_REPO_URL").unwrap_or_else(|_| BASE_REPO_URL.to_string());
    let repo_brach = env::var("STARTER_REPO_BRANCH").unwrap_or_else(|_| BRANCH_NAME.to_string());

    tracing::debug!(
        repo_url = repo_url,
        clone_folder = temp_clone_dir.display().to_string(),
        "cloning loco"
    );

    // We prioritize cloning the Loco project directly from the Git binary if it is installed,
    // to avoid potential conflicts with custom local Git settings, such as 'insteadOf'.
    // If Git is not installed, an alternative approach is attempting to clone the repository using the 'git2' library.
    if git_exists() {
        let args = vec!["clone", "--depth=1", "-b", &repo_brach, &repo_url];
        Command::new("git")
            .args(&args)
            .arg(&temp_clone_dir)
            .output()?;
    } else {
        cfg_if::cfg_if! {
            if #[cfg(feature = "git2")] {
                clone_repo_with_git2(&temp_clone_dir)?;
            } else if #[cfg(feature = "gix")] {
                clone_repo_with_gix(&temp_clone_dir)?;
            } else {
                eyre::bail!("git command is not found. Either install it, or enable git2 or gix feature for this CLI.");
            }
        }
    }

    Ok(temp_clone_dir)
}

fn git_exists() -> bool {
    match Command::new("git").arg("--version").output() {
        Ok(p) => p.status.success(),
        Err(err) => {
            tracing::debug!(error = err.to_string(), "git not found");
            false
        }
    }
}

#[cfg(feature = "git2")]
fn clone_repo_with_git2(temp_clone_dir: &Path) -> eyre::Result<()> {
    let mut fetch_options = git2::FetchOptions::new();
    fetch_options.depth(1);
    git2::build::RepoBuilder::new()
        .fetch_options(fetch_options)
        .clone(BASE_REPO_URL, temp_clone_dir)?;
    Ok(())
}

#[cfg(feature = "gix")]
fn clone_repo_with_gix(temp_clone_dir: &Path) -> eyre::Result<()> {
    let mut prepare_clone = gix::prepare_clone(BASE_REPO_URL, temp_clone_dir)?;
    let (mut prepare_checkout, _outcome) = prepare_clone
        .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    let (_repo, _outcome) =
        prepare_checkout.main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    Ok(())
}
