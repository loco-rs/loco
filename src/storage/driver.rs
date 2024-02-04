use super::error::StoreResult;
use bytes::Bytes;
pub use object_store;
use object_store::ObjectStore;
use std::path::Path;
use std::sync::Arc;

/// Object Store struct that interacts with a specific implementation of `ObjectStore`.
#[derive(Clone)]
pub struct Store {
    driver: Arc<dyn object_store::ObjectStore>,
}

/// Constructor for creating a new `Store` instance.
pub fn new(driver: Arc<dyn ObjectStore>) -> Store {
    Store { driver }
}

impl Store {
    /// Uploads the content represented by `Bytes` to the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StoreResult` with the result of the upload operation.
    pub async fn upload(
        &self,
        path: &Path,
        content: &Bytes,
    ) -> StoreResult<object_store::PutResult> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.driver.put(&path, content.clone()).await?)
    }

    /// Retrieves the content from the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StoreResult` with the result of the retrieval operation.
    pub async fn get(&self, path: &Path) -> StoreResult<object_store::GetResult> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.driver.get(&path).await?)
    }

    /// Deletes the content at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StoreResult` indicating the success of the deletion operation.
    pub async fn delete(&self, path: &Path) -> StoreResult<()> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.driver.delete(&path).await?)
    }

    /// Renames or moves the content from one path to another in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StoreResult` indicating the success of the rename/move operation.
    pub async fn rename(&self, from: &Path, to: &Path) -> StoreResult<()> {
        let from = object_store::path::Path::from(from.display().to_string());
        let to = object_store::path::Path::from(to.display().to_string());
        Ok(self.driver.rename(&from, &to).await?)
    }

    /// Copies the content from one path to another in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StoreResult` indicating the success of the copy operation.
    pub async fn copy(&self, from: &Path, to: &Path) -> StoreResult<()> {
        let from = object_store::path::Path::from(from.display().to_string());
        let to = object_store::path::Path::from(to.display().to_string());
        Ok(self.driver.copy(&from, &to).await?)
    }

    /// Checks if the content exists at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StoreResult` with a boolean indicating the existence of the content.
    pub async fn exists(&self, path: &Path) -> StoreResult<bool> {
        let path = object_store::path::Path::from(path.display().to_string());
        Ok(self.driver.get(&path).await.is_ok())
    }
}
