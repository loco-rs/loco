use object_store::{local::LocalFileSystem, ObjectStore};

use super::Store;
use crate::Result;

/// Create new filesystem storage with no prefix
#[must_use]
pub fn new() -> Store {
    Store::new((Box::new(LocalFileSystem::new()) as Box<dyn ObjectStore>).into())
}

/// Create new filesystem storage with `prefix` applied to all paths
///
/// # Errors
///
/// Returns an error if the path does not exist
pub fn new_with_prefix(prefix: impl AsRef<std::path::Path>) -> Result<Store> {
    Ok(Store::new(
        (Box::new(LocalFileSystem::new_with_prefix(prefix).map_err(Box::from)?)
            as Box<dyn object_store::ObjectStore>)
            .into(),
    ))
}
