use object_store::local::LocalFileSystem;

use super::{object_store_adapter::ObjectStoreAdapter, StoreDriver};
use crate::Result;

/// Create new filesystem storage with no prefix
///
/// # Examples
///```
/// use loco_rs::storage::drivers::local;
/// let file_system_driver = local::new();
/// ```
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    Box::new(ObjectStoreAdapter::new(Box::new(LocalFileSystem::new())))
}

/// Create new filesystem storage with `prefix` applied to all paths
///
/// # Examples
///```
/// use loco_rs::storage::drivers::local;
/// let file_system_driver = local::new_with_prefix("users");
/// ```
///
/// # Errors
///
/// Returns an error if the path does not exist
pub fn new_with_prefix(prefix: impl AsRef<std::path::Path>) -> Result<Box<dyn StoreDriver>> {
    Ok(Box::new(ObjectStoreAdapter::new(Box::new(
        LocalFileSystem::new_with_prefix(prefix).map_err(Box::from)?,
    ))))
}
