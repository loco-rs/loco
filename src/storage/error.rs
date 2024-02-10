use std::{collections::BTreeMap, path::PathBuf};

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum StorageError {
    #[error("store not found by the given key: {0}")]
    StoreNotFound(String),

    #[error(transparent)]
    Store(#[from] object_store::Error),

    #[error("Unable to read data from file {}", path.display().to_string())]
    UnableToReadBytes { path: PathBuf },

    #[error("secondaries errors")]
    Multi(BTreeMap<String, String>),
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;
