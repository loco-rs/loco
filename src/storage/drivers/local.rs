use opendal::{Operator, services::Fs};

use super::StoreDriver;
use crate::storage::{StorageResult, drivers::opendal_adapter::OpendalAdapter};

/// Create new filesystem storage with no prefix
///
/// # Examples
///```
/// use loco_rs::storage::drivers::local;
/// let file_system_driver = local::new();
/// ```
///
/// # Panics
///
/// Panics if the filesystem service built failed.
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    let fs = Fs::default().root("/");
    Box::new(OpendalAdapter::new(
        Operator::new(fs)
            .expect("fs service should build with success")
            .finish(),
    ))
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
pub fn new_with_prefix(prefix: impl AsRef<std::path::Path>) -> StorageResult<Box<dyn StoreDriver>> {
    let fs = Fs::default().root(&prefix.as_ref().display().to_string());
    Ok(Box::new(OpendalAdapter::new(Operator::new(fs)?.finish())))
}
