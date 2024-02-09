use object_store::local::LocalFileSystem;

use super::{object_store_adapter::ObjectStoreAdapter, StoreDriver};
use crate::Result;

/// Create new filesystem storage with no prefix
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    Box::new(ObjectStoreAdapter::new(Box::new(LocalFileSystem::new())))
}

/// Create new filesystem storage with `prefix` applied to all paths
///
/// # Errors
///
/// Returns an error if the path does not exist
pub fn new_with_prefix(prefix: impl AsRef<std::path::Path>) -> Result<Box<dyn StoreDriver>> {
    Ok(Box::new(ObjectStoreAdapter::new(Box::new(
        LocalFileSystem::new_with_prefix(prefix).map_err(Box::from)?,
    ))))
}
