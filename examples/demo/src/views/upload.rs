use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    pub path: PathBuf,
}

impl Response {
    #[must_use]
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }
}
