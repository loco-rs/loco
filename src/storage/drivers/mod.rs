use std::{path::Path, sync::Arc};

use bytes::Bytes;
#[cfg(feature = "storage_aws_s3")]
pub mod aws;
#[cfg(feature = "storage_azure")]
pub mod azure;
#[cfg(feature = "storage_gcp")]
pub mod gcp;
pub mod local;
pub mod mem;
pub use object_store;
use object_store::ObjectStore;

use super::error::StorageResult;

#[derive(Clone)]
pub struct Store {
    driver: Arc<dyn object_store::ObjectStore>,
}

impl Store {
    /// Constructor for creating a new `Store` instance.
    pub fn new(driver: Arc<dyn ObjectStore>) -> Self {
        Self { driver }
    }
}

impl Store {
    /// Uploads the content represented by `Bytes` to the specified path in the
    /// object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the upload operation.
    pub async fn upload(
        &self,
        path: &Path,
        content: &Bytes,
    ) -> StorageResult<object_store::PutResult> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.driver.put(&path, content.clone()).await?)
    }

    /// Retrieves the content from the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the retrieval operation.
    pub async fn get(&self, path: &Path) -> StorageResult<object_store::GetResult> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.driver.get(&path).await?)
    }

    /// Deletes the content at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the deletion
    /// operation.
    pub async fn delete(&self, path: &Path) -> StorageResult<()> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.driver.delete(&path).await?)
    }

    /// Renames or moves the content from one path to another in the object
    /// store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the rename/move
    /// operation.
    pub async fn rename(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let from = object_store::path::Path::from(from.display().to_string());
        let to = object_store::path::Path::from(to.display().to_string());
        Ok(self.driver.rename(&from, &to).await?)
    }

    /// Copies the content from one path to another in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the copy operation.
    pub async fn copy(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let from = object_store::path::Path::from(from.display().to_string());
        let to = object_store::path::Path::from(to.display().to_string());
        Ok(self.driver.copy(&from, &to).await?)
    }

    /// Checks if the content exists at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with a boolean indicating the existence of the
    /// content.
    pub async fn exists(&self, path: &Path) -> StorageResult<bool> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.driver.get(&path).await.is_ok())
    }
}
