//! # Null Storage Driver
//!
//! The Null storage Driver is the default storage driver implemented when the
//! Loco framework is initialized. The primary purpose of this driver is to
//! simplify the user workflow by avoiding the need for feature flags or
//! optional storage driver configurations.
use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;

use super::{GetResponse, ListEntry, StorageResult, StoreDriver, UploadResponse};
use crate::storage::StorageError;

pub struct NullStorage {}

/// Constructor for creating a new `Store` instance.
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    Box::new(NullStorage {})
}

#[async_trait]
impl StoreDriver for NullStorage {
    /// Uploads the content represented by `Bytes` to the specified path in the
    /// object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the upload operation.
    async fn upload(&self, _path: &Path, _content: &Bytes) -> StorageResult<UploadResponse> {
        Err(StorageError::Any(
            "Operation not supported by null storage".into(),
        ))
    }

    /// Retrieves the content from the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the retrieval operation.
    async fn get(&self, _path: &Path) -> StorageResult<GetResponse> {
        Err(StorageError::Any(
            "Operation not supported by null storage".into(),
        ))
    }

    /// Deletes the content at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the deletion
    /// operation.
    async fn delete(&self, _path: &Path) -> StorageResult<()> {
        Err(StorageError::Any(
            "Operation not supported by null storage".into(),
        ))
    }

    /// Renames or moves the content from one path to another in the object
    /// store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the rename/move
    /// operation.
    async fn rename(&self, _from: &Path, _to: &Path) -> StorageResult<()> {
        Err(StorageError::Any(
            "Operation not supported by null storage".into(),
        ))
    }

    /// Copies the content from one path to another in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the copy operation.
    async fn copy(&self, _from: &Path, _to: &Path) -> StorageResult<()> {
        Err(StorageError::Any(
            "Operation not supported by null storage".into(),
        ))
    }

    /// Checks if the content exists at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with a boolean indicating the existence of the
    /// content.
    async fn exists(&self, _path: &Path) -> StorageResult<bool> {
        Err(StorageError::Any(
            "Operation not supported by null storage".into(),
        ))
    }

    /// Lists entries whose paths start with the given prefix.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the listing operation's result.
    async fn list(&self, _path: &Path, _recursive: bool) -> StorageResult<Vec<ListEntry>> {
        Err(StorageError::Any(
            "Operation not supported by null storage".into(),
        ))
    }

    /// Retrieves metadata for a single path without downloading its content.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the entry's metadata.
    async fn stat(&self, _path: &Path) -> StorageResult<ListEntry> {
        Err(StorageError::Any(
            "Operation not supported by null storage".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::new;

    /// The null driver's contract is that *every* operation refuses rather than
    /// pretending to succeed. `list` returning `Ok(vec![])` or `exists`
    /// returning `Ok(false)` would each read as "the store is empty" to a
    /// caller, and under a mirror strategy an empty listing is specifically
    /// treated as a miss to fall back from — so a silent `Ok` here would be
    /// indistinguishable from a real one.
    #[tokio::test]
    async fn every_read_refuses_rather_than_reporting_emptiness() {
        let store = new();
        let path = Path::new("anything.txt");

        assert!(store.exists(path).await.is_err());
        assert!(store.list(path, true).await.is_err());
        assert!(store.list(path, false).await.is_err());
        assert!(store.stat(path).await.is_err());
    }
}
