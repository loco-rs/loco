use std::{fs, path::PathBuf};

use cargo_generate::{generate, GenerateArgs, TemplatePath, Vcs};

use crate::template::Starter;

/// A generator form the git repo
///
/// # Errors
/// 1. when `folder_name` does not exist or A non-final component in path is not
///    a directory
/// 2. When has an error generating the git repository
///
/// # Examples
///
/// ```rust
/// use std::path::PathBuf;
/// use rustyrails_cli::template::Starter;
/// let path = PathBuf::from(".");
/// rustyrails_cli::generate::demo_site(&Starter::Saas,&path, "demo-website", None);
/// ```
pub fn demo_site(
    starter_template: &Starter,
    path: &PathBuf,
    folder_name: &str,
    define: Option<Vec<String>>,
) -> eyre::Result<PathBuf> {
    let define = define.unwrap_or_default();

    let args = GenerateArgs {
        destination: Some(fs::canonicalize(path)?),
        name: Some(folder_name.to_string()),
        vcs: Some(Vcs::Git),
        template_path: TemplatePath {
            git: Some(starter_template.git_url()),
            ..TemplatePath::default()
        },
        define,
        ..GenerateArgs::default()
    };

    generate(args).map_err(|e| eyre::eyre!(e))
}
