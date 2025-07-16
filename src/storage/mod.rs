//! # Storage Module
//!
//! This module defines a generic storage abstraction represented by the
//! [`Storage`] struct. It provides methods for performing common storage
//! operations such as upload, download, delete, rename, and copy.
//!
//! ## Storage Strategy
//!
//! The [`Storage`] struct is designed to work with different storage
//! strategies. A storage strategy defines the behavior of the storage
//! operations. Strategies implement the [`strategies::StorageStrategy`].
//! The selected strategy can be dynamically changed at runtime.
mod contents;
pub mod drivers;
pub mod strategies;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use bytes::Bytes;

use self::drivers::StoreDriver;

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum StorageError {
    #[error("store not found by the given key: {0}")]
    StoreNotFound(String),

    #[error(transparent)]
    Store(#[from] Box<opendal::Error>),

    #[error("Unable to read data from file {}", path.display().to_string())]
    UnableToReadBytes { path: PathBuf },

    #[error("secondaries errors")]
    Multi(BTreeMap<String, String>),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

impl From<opendal::Error> for StorageError {
    fn from(val: opendal::Error) -> Self {
        Self::Store(Box::new(val))
    }
}

pub struct Storage {
    pub stores: BTreeMap<String, Box<dyn StoreDriver>>,
    pub strategy: Box<dyn strategies::StorageStrategy>,
}

impl Storage {
    /// Creates a new storage instance with a single store and the default
    /// strategy.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    ///
    /// let storage = storage::Storage::single(storage::drivers::mem::new());
    /// ```
    #[must_use]
    pub fn single(store: Box<dyn StoreDriver>) -> Self {
        let default_key = "store";
        Self {
            strategy: Box::new(strategies::single::SingleStrategy::new(default_key)),
            stores: BTreeMap::from([(default_key.to_string(), store)]),
        }
    }

    /// Creates a new storage instance with the provided stores and strategy.
    #[must_use]
    pub fn new(
        stores: BTreeMap<String, Box<dyn StoreDriver>>,
        strategy: Box<dyn strategies::StorageStrategy>,
    ) -> Self {
        Self { stores, strategy }
    }

    /// Uploads content to the storage at the specified path.
    ///
    /// This method uses the selected strategy for the upload operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn upload() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("example.txt");
    ///     let content = "Loco!";
    ///     let result = storage.upload(path, &Bytes::from(content)).await;
    ///     assert!(result.is_ok());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the upload operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn upload(&self, path: &Path, content: &Bytes) -> StorageResult<()> {
        self.upload_with_strategy(path, content, &*self.strategy)
            .await
    }

    /// Uploads content to the storage at the specified path using a specific
    /// strategy.
    ///
    /// This method allows specifying a custom strategy for the upload
    /// operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the upload operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn upload_with_strategy(
        &self,
        path: &Path,
        content: &Bytes,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<()> {
        strategy.upload(self, path, content).await
    }

    /// Downloads content from the storage at the specified path.
    ///
    /// This method uses the selected strategy for the download operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn download() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("example.txt");
    ///     let content = "Loco!";
    ///     storage.upload(path, &Bytes::from(content)).await;
    ///
    ///     let result: String = storage.download(path).await.unwrap();
    ///     assert_eq!(result, "Loco!");
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the download operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn download<T: TryFrom<contents::Contents>>(&self, path: &Path) -> StorageResult<T> {
        self.download_with_policy(path, &*self.strategy).await
    }

    /// Downloads content from the storage at the specified path using a
    /// specific strategy.
    ///
    /// This method allows specifying a custom strategy for the download
    /// operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the download operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn download_with_policy<T: TryFrom<contents::Contents>>(
        &self,
        path: &Path,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<T> {
        let res = strategy.download(self, path).await?;
        contents::Contents::from(res).try_into().map_or_else(
            |_| {
                Err(StorageError::UnableToReadBytes {
                    path: path.to_path_buf(),
                })
            },
            |content| Ok(content),
        )
    }

    /// Deletes content from the storage at the specified path.
    ///
    /// This method uses the selected strategy for the delete operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn download() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("example.txt");
    ///     let content = "Loco!";
    ///     storage.upload(path, &Bytes::from(content)).await;
    ///
    ///     let result = storage.delete(path).await;
    ///     assert!(result.is_ok());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the delete operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn delete(&self, path: &Path) -> StorageResult<()> {
        self.delete_with_policy(path, &*self.strategy).await
    }

    /// Deletes content from the storage at the specified path using a specific
    /// strategy.
    ///
    /// This method allows specifying a custom strategy for the delete
    /// operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the delete operation fails or if there
    /// is an issue with the strategy configuration.    
    pub async fn delete_with_policy(
        &self,
        path: &Path,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<()> {
        strategy.delete(self, path).await
    }

    /// Renames content from one path to another in the storage.
    ///
    /// This method uses the selected strategy for the rename operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn download() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("example.txt");
    ///     let content = "Loco!";
    ///     storage.upload(path, &Bytes::from(content)).await;
    ///     
    ///     let new_path = Path::new("new_path.txt");
    ///     let store = storage.as_store("default").unwrap();
    ///     assert!(storage.rename(&path, &new_path).await.is_ok());
    ///     assert!(!store.exists(&path).await.unwrap());
    ///     assert!(store.exists(&new_path).await.unwrap());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the rename operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn rename(&self, from: &Path, to: &Path) -> StorageResult<()> {
        self.rename_with_policy(from, to, &*self.strategy).await
    }

    /// Renames content from one path to another in the storage using a specific
    /// strategy.
    ///
    /// This method allows specifying a custom strategy for the rename
    /// operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the rename operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn rename_with_policy(
        &self,
        from: &Path,
        to: &Path,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<()> {
        strategy.rename(self, from, to).await
    }

    /// Copies content from one path to another in the storage.
    ///
    /// This method uses the selected strategy for the copy operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn download() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("example.txt");
    ///     let content = "Loco!";
    ///     storage.upload(path, &Bytes::from(content)).await;
    ///     
    ///     let new_path = Path::new("new_path.txt");
    ///     let store = storage.as_store("default").unwrap();
    ///     assert!(storage.copy(&path, &new_path).await.is_ok());
    ///     assert!(store.exists(&path).await.unwrap());
    ///     assert!(store.exists(&new_path).await.unwrap());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the copy operation fails or if there is
    /// an issue with the strategy configuration.
    pub async fn copy(&self, from: &Path, to: &Path) -> StorageResult<()> {
        self.copy_with_policy(from, to, &*self.strategy).await
    }

    /// Copies content from one path to another in the storage using a specific
    /// strategy.
    ///
    /// This method allows specifying a custom strategy for the copy operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the copy operation fails or if there is
    /// an issue with the strategy configuration.
    pub async fn copy_with_policy(
        &self,
        from: &Path,
        to: &Path,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<()> {
        strategy.copy(self, from, to).await
    }

    /// Returns a reference to the store with the specified name if exists.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn download() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     assert!(storage.as_store("default").is_some());
    ///     assert!(storage.as_store("store_2").is_none());
    /// }
    /// ```
    ///
    /// # Returns
    /// Return None if the given name not found.
    #[must_use]
    pub fn as_store(&self, name: &str) -> Option<&dyn StoreDriver> {
        self.stores.get(name).map(|s| &**s)
    }

    /// Returns a reference to the store with the specified name.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn download() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     assert!(storage.as_store_err("default").is_ok());
    ///     assert!(storage.as_store_err("store_2").is_err());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Return an error if the given store name not exists
    // REVIEW(nd): not sure bout the name 'as_store_err' -- it returns result
    pub fn as_store_err(&self, name: &str) -> StorageResult<&dyn StoreDriver> {
        self.as_store(name)
            .ok_or(StorageError::StoreNotFound(name.to_string()))
    }
}
