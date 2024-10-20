//! # Null Storage Driver
//!
//! The Null storage Driver is the default storage driver implemented when the
//! Loco framework is initialized. The primary purpose of this driver is to
//! simplify the user workflow by avoiding the need for feature flags or
//! optional storage driver configurations.
use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;

use super::{GetResponse, StorageResult, StoreDriver, UploadResponse};
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
}
