use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;
use object_store::ObjectStore;

use super::{GetResponse, StoreDriver, UploadResponse};
use crate::storage::StorageResult;

pub struct ObjectStoreAdapter {
    object_store_impl: Box<dyn object_store::ObjectStore>,
}

impl ObjectStoreAdapter {
    /// Constructor for creating a new `Store` instance.
    #[must_use]
    pub fn new(object_store_impl: Box<dyn ObjectStore>) -> Self {
        Self { object_store_impl }
    }
}

#[async_trait]
impl StoreDriver for ObjectStoreAdapter {
    /// Uploads the content represented by `Bytes` to the specified path in the
    /// object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the upload operation.
    async fn upload(&self, path: &Path, content: &Bytes) -> StorageResult<UploadResponse> {
        let path = object_store::path::Path::from(path.display().to_string());
        let res = self.object_store_impl.put(&path, content.clone()).await?;
        Ok(UploadResponse {
            object_uri: res.e_tag,
            version: res.version,
        })
    }

    /// Retrieves the content from the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the retrieval operation.
    async fn get(&self, path: &Path) -> StorageResult<GetResponse> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.object_store_impl.get(&path).await?)
    }

    /// Deletes the content at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the deletion
    /// operation.
    async fn delete(&self, path: &Path) -> StorageResult<()> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.object_store_impl.delete(&path).await?)
    }

    /// Renames or moves the content from one path to another in the object
    /// store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the rename/move
    /// operation.
    async fn rename(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let from = object_store::path::Path::from(from.display().to_string());
        let to = object_store::path::Path::from(to.display().to_string());
        Ok(self.object_store_impl.rename(&from, &to).await?)
    }

    /// Copies the content from one path to another in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the copy operation.
    async fn copy(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let from = object_store::path::Path::from(from.display().to_string());
        let to = object_store::path::Path::from(to.display().to_string());
        Ok(self.object_store_impl.copy(&from, &to).await?)
    }

    /// Checks if the content exists at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with a boolean indicating the existence of the
    /// content.
    async fn exists(&self, path: &Path) -> StorageResult<bool> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.object_store_impl.get(&path).await.is_ok())
    }
}
