use std::{fs, path::PathBuf};

use cargo_generate::{generate, GenerateArgs, TemplatePath, Vcs};

/// A generator form the git repo
///
/// # Errors
/// Returns error when cannot generate
pub fn new_project(
    starter_url: &String,
    path: &PathBuf,
    app: &str,
    random_string: &str,
) -> eyre::Result<PathBuf> {
    let mut define = vec![format!("auth_secret={random_string}")];
    define.push(format!("lib_name={app}"));

    let args = GenerateArgs {
        destination: Some(fs::canonicalize(path)?),
        name: Some(app.to_string()),
        vcs: Some(Vcs::Git),
        template_path: TemplatePath {
            branch: None,
            git: Some(starter_url.to_string()),
            ..TemplatePath::default()
        },
        define,
        ..GenerateArgs::default()
    };

    generate(args).map_err(|e| eyre::eyre!(e))
}
