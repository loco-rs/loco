use std::fs;
use std::path::{Path, PathBuf};

pub const FOLDER_EXAMPLES: &str = "examples";
pub const FOLDER_STARTERS: &str = "starters";
pub const FOLDER_LOCO_CLI: &str = "loco-cli";

/// return a lost of cargo project in the given path
///
/// # Errors
/// when could not read the given dir path
pub fn get_cargo_folders(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let paths = fs::read_dir(path)?;
    Ok(paths
        .filter_map(std::result::Result::ok)
        .filter_map(|dir| {
            if dir.path().join("Cargo.toml").exists() {
                Some(dir.path())
            } else {
                None
            }
        })
        .collect())
}
