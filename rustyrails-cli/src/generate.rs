use std::{fs, path::PathBuf};

use cargo_generate::{generate, GenerateArgs, TemplatePath, Vcs};

/// Generator github template
const RUSTYRAILS_DEMO_TEMPLATE: &str =
    "https://github.com/rustyrails-rs/rustyrails-starter-template";

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
/// let path = PathBuf::from(".");
/// rustyrails_cli::generate::demo_site(&path, "demo-website", None);
/// ```
pub fn demo_site(
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
            git: Some(RUSTYRAILS_DEMO_TEMPLATE.to_string()),
            ..TemplatePath::default()
        },
        define,
        ..GenerateArgs::default()
    };

    generate(args).map_err(|e| eyre::eyre!(e))
}
