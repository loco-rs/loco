mod contents;
pub mod driver;
pub mod error;
use bytes::Bytes;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Enum representing different storage configurations.
pub enum Kind {
    Single(driver::Store),
    Mirror(driver::Store, Vec<driver::Store>, MirrorPolicy),
    Multi(driver::Store, HashMap<String, driver::Store>, MirrorPolicy),
}

/// Implementation for the store initializer options.
impl Kind {
    /// Builds a `Storage` instance based on the specified configuration.
    #[must_use]
    pub fn build(&self) -> Storage {
        match self {
            Self::Single(primary) => Storage::with_single(primary),
            Self::Mirror(primary, stores, policy) => Storage::with_mirror(primary, stores, policy),
            Self::Multi(primary, stores, policy) => Storage::with_multi(primary, stores, policy),
        }
    }
}

/// Enum representing mirror policies for the `Storage`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirrorPolicy {
    ContinueOnFailure,
    StopOnFailure,

    None,
}

/// Struct representing a storage configuration.
#[derive(Clone)]
pub struct Storage {
    primary: driver::Store,
    mirror_policy: MirrorPolicy,
    stores: HashMap<String, driver::Store>,
}

/// Implementation for the `Storage` struct.
impl Storage {
    /// Creates a `Storage` instance with a single primary store.
    #[must_use]
    pub fn with_single(primary: &driver::Store) -> Self {
        Self {
            primary: primary.clone(),
            mirror_policy: MirrorPolicy::None,
            stores: HashMap::new(),
        }
    }

    /// Creates a `Storage` instance with a primary store and a list of mirror stores.
    #[must_use]
    pub fn with_mirror(
        primary: &driver::Store,
        stores: &[driver::Store],
        policy: &MirrorPolicy,
    ) -> Self {
        let hashmap = stores
            .iter()
            .enumerate()
            .map(|(index, value)| (format!("{}", index + 1), value.clone()))
            .collect();

        Self {
            primary: primary.clone(),
            mirror_policy: policy.clone(),
            stores: hashmap,
        }
    }

    /// Creates a `Storage` instance with a primary store and a map of named mirror stores.
    #[must_use]
    pub fn with_multi(
        primary: &driver::Store,
        stores: &HashMap<String, driver::Store>,
        policy: &MirrorPolicy,
    ) -> Self {
        Self {
            primary: primary.clone(),
            mirror_policy: policy.clone(),
            stores: stores.clone(),
        }
    }

    /// Uploads content to the specified path in the storage.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the upload operation.
    pub async fn upload(
        &self,
        path: &Path,
        content: &Bytes,
    ) -> error::StorageResult<object_store::PutResult> {
        let res = self.primary.upload(path, content).await?;
        let mut error_stores = BTreeMap::new();

        match self.mirror_policy {
            MirrorPolicy::ContinueOnFailure | MirrorPolicy::StopOnFailure => {
                for (store_name, store) in &self.stores {
                    if let Err(error) = store.upload(path, content).await {
                        if !self.handle_error_policy(store_name, &error, &mut error_stores) {
                            return Err(error::StorageError::Mirror(BTreeMap::from([(
                                (*store_name).to_string(),
                                format!("{error}"),
                            )])));
                        }
                    }
                }
                Ok(res)
            }
            MirrorPolicy::None => Ok(res),
        }
    }

    /// Downloads content from the specified path in the storage.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the retrieval operation.
    pub async fn download<T: TryFrom<contents::Contents>>(
        &self,
        path: &Path,
    ) -> error::StorageResult<T> {
        let bytes: Bytes = self
            .primary
            .get(path)
            .await?
            .bytes()
            .await
            .map_err(|e| error::StorageError::Storage(error::StoreError::Storage(e)))?;

        contents::Contents::from(bytes).try_into().map_or_else(
            |_| {
                Err(error::StorageError::Storage(
                    error::StoreError::UnableToReadBytes {
                        path: path.to_path_buf(),
                    },
                ))
            },
            |content| Ok(content),
        )
    }

    /// Checks if content exists at the specified path in the storage.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with a boolean indicating the existence of the content.
    pub async fn exists(&self, path: &Path) -> error::StorageResult<bool> {
        Ok(self.primary.exists(path).await?)
    }

    /// Deletes content at the specified path in the storage.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the deletion operation.
    pub async fn delete(&self, path: &Path) -> error::StorageResult<()> {
        self.primary.delete(path).await?;
        let mut error_stores = BTreeMap::new();

        match self.mirror_policy {
            MirrorPolicy::ContinueOnFailure | MirrorPolicy::StopOnFailure => {
                for (store_name, store) in &self.stores {
                    if let Err(error) = store.delete(path).await {
                        if !self.handle_error_policy(store_name, &error, &mut error_stores) {
                            return Err(error::StorageError::Mirror(BTreeMap::from([(
                                (*store_name).to_string(),
                                format!("{error}"),
                            )])));
                        }
                    }
                }
                Ok(())
            }
            MirrorPolicy::None => Ok(()),
        }
    }

    /// Renames or moves content from one path to another in the storage.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the rename/move operation.
    pub async fn rename(&self, from: &Path, to: &Path) -> error::StorageResult<()> {
        self.primary.rename(from, to).await?;
        let mut error_stores = BTreeMap::new();

        match self.mirror_policy {
            MirrorPolicy::ContinueOnFailure | MirrorPolicy::StopOnFailure => {
                for (store_name, store) in &self.stores {
                    if let Err(error) = store.rename(from, to).await {
                        if !self.handle_error_policy(store_name, &error, &mut error_stores) {
                            return Err(error::StorageError::Mirror(BTreeMap::from([(
                                (*store_name).to_string(),
                                format!("{error}"),
                            )])));
                        }
                    }
                }
                Ok(())
            }
            MirrorPolicy::None => Ok(()),
        }
    }

    /// Copies content from one path to another in the storage.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the copy operation.
    pub async fn copy(&self, from: &Path, to: &Path) -> error::StorageResult<()> {
        self.primary.copy(from, to).await?;
        let mut error_stores = BTreeMap::new();

        match self.mirror_policy {
            MirrorPolicy::ContinueOnFailure | MirrorPolicy::StopOnFailure => {
                for (store_name, store) in &self.stores {
                    if let Err(error) = store.copy(from, to).await {
                        if !self.handle_error_policy(store_name, &error, &mut error_stores) {
                            return Err(error::StorageError::Mirror(BTreeMap::from([(
                                (*store_name).to_string(),
                                format!("{error}"),
                            )])));
                        }
                    }
                }
                Ok(())
            }
            MirrorPolicy::None => Ok(()),
        }
    }

    /// Retrieves a reference to the mirror store with the specified name.
    ///
    /// # Returns
    ///
    /// Returns an `Option` containing a reference to the mirror store if found.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&driver::Store> {
        self.stores.get(name)
    }

    /// Handles the error policy based on the mirror policy and returns a boolean.
    ///
    /// # Returns
    ///
    /// Returns a boolean indicating whether to continue processing or stop based on the error policy.
    fn handle_error_policy(
        &self,
        store_name: &str,
        error: &error::StoreError,
        error_stores: &mut BTreeMap<String, String>,
    ) -> bool {
        match self.mirror_policy {
            MirrorPolicy::ContinueOnFailure => {
                error_stores.insert((*store_name).to_string(), format!("{error}"));
                true
            }
            MirrorPolicy::StopOnFailure => false,
            MirrorPolicy::None => true,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use std::path::PathBuf;

    fn new_storage() -> Storage {
        let driver: Box<dyn object_store::ObjectStore> =
            Box::new(object_store::memory::InMemory::new()) as Box<dyn object_store::ObjectStore>;

        Kind::Single(driver::new(driver.into())).build()
    }

    #[tokio::test]
    async fn can_upload() {
        let storage = new_storage();

        let path = PathBuf::from("storage").join("users");

        let file = path.join("user-1");
        assert!(storage
            .upload(file.as_path(), &Bytes::from("test file upload"))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn can_read_upload_file() {
        let storage = new_storage();
        let file_path = PathBuf::from("storage").join("users").join("user-1");

        assert!(storage
            .upload(file_path.as_path(), &Bytes::from("test file upload"))
            .await
            .is_ok());

        let read_file: error::StorageResult<String> = storage.download(file_path.as_path()).await;
        assert_eq!(read_file.unwrap(), "test file upload".to_string());
    }

    #[tokio::test]
    async fn can_delete() {
        let storage = new_storage();
        let file_path = PathBuf::from("storage").join("users").join("user-1");

        assert!(storage
            .upload(file_path.as_path(), &Bytes::from("test file upload"))
            .await
            .is_ok());

        assert!(storage.delete(file_path.as_path()).await.is_ok());
        assert!(!storage.exists(file_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn can_rename() {
        let storage = new_storage();
        let path = PathBuf::from("storage").join("users");

        let file = path.join("user-1");
        let rename_file = path.join("user-2");

        // make sure rename file not exists
        assert!(!storage.exists(rename_file.as_path()).await.unwrap());

        assert!(storage
            .upload(file.as_path(), &Bytes::from("test file upload"))
            .await
            .is_ok());

        assert!(storage.exists(file.as_path()).await.unwrap());

        assert!(storage
            .rename(file.as_path(), rename_file.as_path())
            .await
            .is_ok());

        assert!(!storage.exists(file.as_path()).await.unwrap());
        assert!(storage.exists(rename_file.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn can_copy() {
        let storage = new_storage();
        let path = PathBuf::from("storage").join("users");

        let file = path.join("user-1");
        let rename_file = path.join("user-2");

        // make sure rename file not exists
        assert!(!storage.exists(rename_file.as_path()).await.unwrap());

        assert!(storage
            .upload(file.as_path(), &Bytes::from("test file upload"))
            .await
            .is_ok());

        assert!(storage.exists(file.as_path()).await.unwrap());

        assert!(storage
            .copy(file.as_path(), rename_file.as_path())
            .await
            .is_ok());

        assert!(storage.exists(file.as_path()).await.unwrap());
        assert!(storage.exists(rename_file.as_path()).await.unwrap());
    }
}
