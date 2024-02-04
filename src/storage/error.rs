use object_store::Error;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum StoreError {
    #[error(transparent)]
    Storage(#[from] Error),

    #[error("Unable to read data from file {}", path.display().to_string())]
    UnableToReadBytes { path: PathBuf },
}

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum StorageError {
    #[error(transparent)]
    Storage(#[from] StoreError),

    #[error("task")]
    Mirror(BTreeMap<String, String>),
}

pub type StoreResult<T> = std::result::Result<T, StoreError>;
pub type StorageResult<T> = std::result::Result<T, StorageError>;
