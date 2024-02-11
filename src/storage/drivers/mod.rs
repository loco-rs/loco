use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;
#[cfg(feature = "storage_aws_s3")]
pub mod aws;
#[cfg(feature = "storage_azure")]
pub mod azure;
#[cfg(feature = "storage_gcp")]
pub mod gcp;
pub mod local;
pub mod mem;
pub mod object_store_adapter;

use super::StorageResult;
pub struct UploadResponse {
    pub e_tag: Option<String>,
    pub version: Option<String>,
}

// TODO: need to properly abstract the object_store type in order to not
// strongly depend on it
pub type GetResponse = object_store::GetResult;

#[async_trait]
pub trait StoreDriver: Sync + Send {
    /// Uploads the content represented by `Bytes` to the specified path in the
    /// object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the upload operation.
    async fn upload(&self, path: &Path, content: &Bytes) -> StorageResult<UploadResponse>;

    /// Retrieves the content from the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the retrieval operation.
    async fn get(&self, path: &Path) -> StorageResult<GetResponse>;

    /// Deletes the content at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the deletion
    /// operation.
    async fn delete(&self, path: &Path) -> StorageResult<()>;

    /// Renames or moves the content from one path to another in the object
    /// store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the rename/move
    /// operation.
    async fn rename(&self, from: &Path, to: &Path) -> StorageResult<()>;

    /// Copies the content from one path to another in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the copy operation.
    async fn copy(&self, from: &Path, to: &Path) -> StorageResult<()>;

    /// Checks if the content exists at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with a boolean indicating the existence of the
    /// content.
    async fn exists(&self, path: &Path) -> StorageResult<bool>;
}
