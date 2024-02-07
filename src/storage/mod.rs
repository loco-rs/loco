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
//! operations. Strategies implement the [`strategies::StorageStrategyTrait`].
//! The selected strategy can be dynamically changed at runtime.
mod contents;
pub mod drivers;
pub mod error;
pub mod strategies;
use std::{collections::BTreeMap, path::Path, sync::Arc};

use bytes::Bytes;

use self::error::StorageResult;

#[derive(Clone)]
pub struct Storage {
    pub stores: BTreeMap<String, drivers::Store>,
    pub strategy: Arc<dyn strategies::StorageStrategyTrait>,
}

impl Storage {
    /// Creates a new storage instance with a single store and the default
    /// strategy.
    #[must_use]
    pub fn single(store: drivers::Store) -> Self {
        let default_key = "store";
        Self {
            strategy: Arc::new(strategies::single::SingleStrategy::new(default_key)),
            stores: BTreeMap::from([(default_key.to_string(), store)]),
        }
    }

    /// Creates a new storage instance with the provided stores and strategy.
    #[must_use]
    pub fn new(
        stores: BTreeMap<String, drivers::Store>,
        strategy: Arc<dyn strategies::StorageStrategyTrait>,
    ) -> Self {
        Self { stores, strategy }
    }

    /// Uploads content to the storage at the specified path.
    ///
    /// This method uses the selected strategy for the upload operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the upload operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn upload(&self, path: &Path, content: &Bytes) -> error::StorageResult<()> {
        self.upload_with_strategy(path, content, &self.strategy)
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
        strategy: &Arc<dyn strategies::StorageStrategyTrait>,
    ) -> error::StorageResult<()> {
        strategy.upload(self, path, content).await
    }

    /// Downloads content from the storage at the specified path.
    ///
    /// This method uses the selected strategy for the download operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the download operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn download<T: TryFrom<contents::Contents>>(
        &self,
        path: &Path,
    ) -> error::StorageResult<T> {
        self.download_with_policy(path, &self.strategy).await
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
        strategy: &Arc<dyn strategies::StorageStrategyTrait>,
    ) -> error::StorageResult<T> {
        let res = strategy.download(self, path).await?;
        contents::Contents::from(res).try_into().map_or_else(
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

    /// Deletes content from the storage at the specified path.
    ///
    /// This method uses the selected strategy for the delete operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the delete operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn delete(&self, path: &Path) -> error::StorageResult<()> {
        self.delete_with_policy(path, &self.strategy).await
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
        strategy: &Arc<dyn strategies::StorageStrategyTrait>,
    ) -> error::StorageResult<()> {
        strategy.delete(self, path).await
    }

    /// Renames content from one path to another in the storage.
    ///
    /// This method uses the selected strategy for the rename operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the rename operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn rename(&self, from: &Path, to: &Path) -> error::StorageResult<()> {
        self.rename_with_policy(from, to, &self.strategy).await
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
        strategy: &Arc<dyn strategies::StorageStrategyTrait>,
    ) -> error::StorageResult<()> {
        strategy.rename(self, from, to).await
    }

    /// Copies content from one path to another in the storage.
    ///
    /// This method uses the selected strategy for the copy operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the copy operation fails or if there is
    /// an issue with the strategy configuration.
    pub async fn copy(&self, from: &Path, to: &Path) -> error::StorageResult<()> {
        self.copy_with_policy(from, to, &self.strategy).await
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
        strategy: &Arc<dyn strategies::StorageStrategyTrait>,
    ) -> error::StorageResult<()> {
        strategy.copy(self, from, to).await
    }

    /// Returns a reference to the store with the specified name if exists.
    #[must_use]
    pub fn as_store(&self, name: &str) -> Option<&drivers::Store> {
        self.stores.get(name)
    }

    /// Returns a reference to the store with the specified name.
    ///
    /// # Errors
    ///
    /// Return an error if the given store name not exists
    pub fn as_store_err(&self, name: &str) -> StorageResult<&drivers::Store> {
        self.as_store(name)
            .ok_or(error::StorageError::StoreNotFound(name.to_string()))
    }
}
