use crate::generate;
use crate::prompt;
use fs_extra::dir::{copy, CopyOptions};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::env;
use std::path::Path;
use std::path::PathBuf;

/// getting logo debug path for working locally.
///
/// # Errors
/// when the path is not given
pub fn debug_path() -> Option<PathBuf> {
    env::var("LOCO_DEBUG_PATH").ok().map(PathBuf::from)
}

const DEFAULT_BRANCH: &str = "master";

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

fn clone_repo() -> Result<PathBuf, git2::Error> {
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();

    let temp_clone_dir = env::temp_dir().join(random_string);

    let from_branch = env::var("LOCO_BRANCH").map_or_else(
        |_| DEFAULT_BRANCH.to_owned(),
        |b| {
            if b.is_empty() {
                DEFAULT_BRANCH.to_string()
            } else {
                b
            }
        },
    );

    tracing::debug!(
        branch = from_branch,
        clone_folder = temp_clone_dir.display().to_string(),
        "cloning loco"
    );
    let mut opt = git2::FetchOptions::new();

    opt.depth(1);

    git2::build::RepoBuilder::new()
        .branch(&from_branch)
        .fetch_options(opt)
        .clone("https://github.com/loco-rs/loco", &temp_clone_dir)?;

    Ok(temp_clone_dir)
}
