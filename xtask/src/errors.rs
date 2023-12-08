use std::path::PathBuf;
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error(
        "could not bump package {} version. not found in path {:?}",
        package,
        path
    )]
    BumpVersion { path: PathBuf, package: String },

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
