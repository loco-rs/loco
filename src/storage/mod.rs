//! # Storage Module
//!
//! This module defines a generic storage abstraction represented by the
//! [`Storage`] struct. It provides methods for performing common storage
//! operations such as upload, download, delete, rename, copy, exists, list,
//! stat, and presigning.
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
pub mod stream;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

use bytes::Bytes;

use self::{
    drivers::{ListEntry, PresignPutOptions, PresignedRequest, StoreDriver},
    stream::BytesStream,
};

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
#[non_exhaustive]
pub enum StorageError {
    #[error("store not found by the given key: {0}")]
    StoreNotFound(String),

    #[cfg(feature = "storage_opendal")]
    #[error(transparent)]
    Store(#[from] Box<opendal::Error>),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Unable to read data from file {}", path.display().to_string())]
    UnableToReadBytes { path: PathBuf },

    #[error("secondaries errors")]
    Multi(BTreeMap<String, String>),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

#[cfg(feature = "storage_opendal")]
impl From<opendal::Error> for StorageError {
    fn from(val: opendal::Error) -> Self {
        Self::Store(Box::new(val))
    }
}

pub struct Storage {
    pub stores: BTreeMap<String, Box<dyn StoreDriver>>,
    pub strategy: Box<dyn strategies::StorageStrategy>,
}

/// Builds a single [`StoreDriver`] from a [`crate::config::StorageConfig`].
///
/// Shared by [`create_storage_provider`] (single default store) and
/// [`create_multi_storage_provider`] (one driver per named store).
fn driver_from_config(cfg: &crate::config::StorageConfig) -> StorageResult<Box<dyn StoreDriver>> {
    use crate::config::StorageConfig;

    Ok(match cfg {
        #[cfg(feature = "storage_aws_s3")]
        StorageConfig::Aws(c) => {
            // An empty string means unset: `${AWS_KEY:-}` style templating
            // yields "" for a missing variable, and "" is never a real key.
            let non_empty = |v: &Option<String>| v.clone().filter(|s| !s.is_empty());
            let endpoint = non_empty(&c.endpoint);
            match (non_empty(&c.access_key_id), non_empty(&c.secret_access_key)) {
                (Some(key_id), Some(secret_key)) => {
                    let credentials = drivers::aws::Credential {
                        key_id,
                        secret_key,
                        token: non_empty(&c.session_token),
                    };
                    match endpoint {
                        Some(endpoint) => drivers::aws::with_credentials_and_endpoint(
                            &c.bucket,
                            &c.region,
                            &endpoint,
                            credentials,
                        )?,
                        None => drivers::aws::with_credentials(&c.bucket, &c.region, credentials)?,
                    }
                }
                _ => match endpoint {
                    Some(endpoint) => drivers::aws::with_endpoint(&c.bucket, &c.region, &endpoint)?,
                    None => drivers::aws::new(&c.bucket, &c.region)?,
                },
            }
        }
        #[cfg(feature = "storage_gcp")]
        StorageConfig::Gcp(c) => drivers::gcp::new(&c.bucket, &c.credential_path)?,
        #[cfg(feature = "storage_azure")]
        StorageConfig::Azure(c) => {
            drivers::azure::new(&c.container, &c.account_name, &c.access_key, &c.endpoint)?
        }
        StorageConfig::Local(c) => match &c.path {
            Some(path) => drivers::local::new_with_prefix(path)?,
            None => drivers::local::new(),
        },
        StorageConfig::Null => drivers::null::new(),
    })
}

/// Builds the default [`Storage`] from `config.storage`.
///
/// Mirrors [`crate::cache::create_cache_provider`]: the `kind`-tagged config
/// selects a driver via its factory function. Runs during
/// [`crate::boot::create_context`], before [`crate::app::Hooks::after_context`]
/// — which still runs afterward and can freely replace the result via
/// `ctx.into_builder().storage(..).build()`, exactly as it does today.
///
/// # Errors
///
/// Returns an error if the selected driver fails to initialize (e.g. an
/// invalid `local` path, or a malformed cloud-provider client).
pub fn create_storage_provider(config: &crate::config::Config) -> StorageResult<Storage> {
    Ok(Storage::single(driver_from_config(&config.storage)?))
}

/// Builds a [`strategies::StorageStrategy`] from a
/// [`crate::config::StorageStrategyConfig`].
fn strategy_from_config(
    cfg: &crate::config::StorageStrategyConfig,
) -> Box<dyn strategies::StorageStrategy> {
    use crate::config::StorageStrategyConfig;
    use strategies::replicated::ReplicatedStrategy;

    match cfg {
        StorageStrategyConfig::Single { default } => {
            Box::new(strategies::single::SingleStrategy::new(default))
        }
        StorageStrategyConfig::Mirror {
            primary,
            secondaries,
            failure_policy,
        } => Box::new(ReplicatedStrategy::mirror(
            primary,
            Some(secondaries.clone()),
            failure_policy.clone(),
        )),
        StorageStrategyConfig::Backup {
            primary,
            secondaries,
            failure_policy,
        } => Box::new(ReplicatedStrategy::backup(
            primary,
            Some(secondaries.clone()),
            failure_policy.clone(),
        )),
    }
}

/// Rejects a strategy that names a store missing from `stores`, names one
/// store twice, or replicates to no secondary. At runtime a missing secondary
/// only counts as a failure, so `FailurePolicy::AllowAll` would hide it:
/// writes succeed, nothing replicates.
fn validate_strategy_stores(config: &crate::config::MultiStorageConfig) -> StorageResult<()> {
    use crate::config::StorageStrategyConfig;

    let (primary, secondaries) = match &config.strategy {
        StorageStrategyConfig::Single { default } => (default, [].as_slice()),
        StorageStrategyConfig::Mirror {
            primary,
            secondaries,
            ..
        }
        | StorageStrategyConfig::Backup {
            primary,
            secondaries,
            ..
        } => {
            if secondaries.is_empty() {
                return Err(StorageError::Any(
                    "a Mirror or Backup storage strategy needs at least one secondary".into(),
                ));
            }
            (primary, secondaries.as_slice())
        }
    };

    let mut seen = std::collections::HashSet::new();
    for name in std::iter::once(primary).chain(secondaries) {
        if !config.stores.contains_key(name) {
            return Err(StorageError::StoreNotFound(name.clone()));
        }
        if !seen.insert(name) {
            return Err(StorageError::Any(
                format!("store `{name}` is listed more than once in the storage strategy").into(),
            ));
        }
    }
    Ok(())
}

/// Builds a multi-store [`Storage`] from a [`crate::config::MultiStorageConfig`].
/// Used by [`crate::initializers::multi_storage::MultiStorageInitializer`].
///
/// # Errors
///
/// Returns an error if the strategy names a store missing from `stores` or
/// names one store twice, if `Mirror`/`Backup` has no secondaries, or if any
/// named store's driver fails to initialize.
pub fn create_multi_storage_provider(
    config: &crate::config::MultiStorageConfig,
) -> StorageResult<Storage> {
    validate_strategy_stores(config)?;
    let stores = config
        .stores
        .iter()
        .map(|(name, cfg)| Ok((name.clone(), driver_from_config(cfg)?)))
        .collect::<StorageResult<BTreeMap<_, _>>>()?;
    Ok(Storage::new(stores, strategy_from_config(&config.strategy)))
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

    /// Checks whether content exists at the specified path.
    ///
    /// This method uses the selected strategy for the exists check.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn check_exists() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("example.txt");
    ///     storage.upload(path, &Bytes::from("Loco!")).await.unwrap();
    ///
    ///     assert!(storage.exists(path).await.unwrap());
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the exists check fails or if there is
    /// an issue with the strategy configuration.
    pub async fn exists(&self, path: &Path) -> StorageResult<bool> {
        self.exists_with_policy(path, &*self.strategy).await
    }

    /// Checks whether content exists at the specified path using a specific
    /// strategy.
    ///
    /// This method allows specifying a custom strategy for the exists check.
    ///
    /// # Errors
    ///
    /// This method returns an error if the exists check fails or if there is
    /// an issue with the strategy configuration.
    pub async fn exists_with_policy(
        &self,
        path: &Path,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<bool> {
        strategy.exists(self, path).await
    }

    /// Lists entries whose paths start with the given prefix.
    ///
    /// This method uses the selected strategy for the listing operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn list_entries() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("dir/example.txt");
    ///     storage.upload(path, &Bytes::from("Loco!")).await.unwrap();
    ///
    ///     let entries = storage.list(Path::new("dir"), true).await.unwrap();
    ///     assert_eq!(entries.len(), 1);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the listing operation fails or if
    /// there is an issue with the strategy configuration.
    pub async fn list(&self, path: &Path, recursive: bool) -> StorageResult<Vec<ListEntry>> {
        self.list_with_policy(path, recursive, &*self.strategy)
            .await
    }

    /// Lists entries whose paths start with the given prefix using a specific
    /// strategy.
    ///
    /// This method allows specifying a custom strategy for the listing
    /// operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the listing operation fails or if
    /// there is an issue with the strategy configuration.
    pub async fn list_with_policy(
        &self,
        path: &Path,
        recursive: bool,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<Vec<ListEntry>> {
        strategy.list(self, path, recursive).await
    }

    /// Retrieves metadata for a single path without downloading its content.
    ///
    /// This method uses the selected strategy for the stat operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// use bytes::Bytes;
    /// pub async fn stat_entry() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("example.txt");
    ///     storage.upload(path, &Bytes::from("Loco!")).await.unwrap();
    ///
    ///     let entry = storage.stat(path).await.unwrap();
    ///     assert_eq!(entry.content_length, Some(5));
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the stat operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn stat(&self, path: &Path) -> StorageResult<ListEntry> {
        self.stat_with_policy(path, &*self.strategy).await
    }

    /// Retrieves metadata for a single path using a specific strategy.
    ///
    /// This method allows specifying a custom strategy for the stat
    /// operation.
    ///
    /// # Errors
    ///
    /// This method returns an error if the stat operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn stat_with_policy(
        &self,
        path: &Path,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<ListEntry> {
        strategy.stat(self, path).await
    }

    /// Builds a presigned `GET` URL for `path` using the selected strategy.
    ///
    /// # Errors
    ///
    /// Returns an error if the driver does not support presigning or the
    /// request could not be built.
    pub async fn presign_get(
        &self,
        path: &Path,
        expire: Duration,
    ) -> StorageResult<PresignedRequest> {
        self.presign_get_with_policy(path, expire, &*self.strategy)
            .await
    }

    /// Builds a presigned `GET` URL using a specific strategy.
    ///
    /// # Errors
    ///
    /// Returns an error if the driver does not support presigning or the
    /// request could not be built.
    pub async fn presign_get_with_policy(
        &self,
        path: &Path,
        expire: Duration,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<PresignedRequest> {
        strategy.presign_get(self, path, expire).await
    }

    /// Builds a presigned `PUT` URL for `path` using the selected strategy.
    ///
    /// # Errors
    ///
    /// Returns an error if the driver does not support presigning or the
    /// request could not be built.
    pub async fn presign_put(
        &self,
        path: &Path,
        expire: Duration,
        options: PresignPutOptions,
    ) -> StorageResult<PresignedRequest> {
        self.presign_put_with_policy(path, expire, options, &*self.strategy)
            .await
    }

    /// Builds a presigned `PUT` URL using a specific strategy.
    ///
    /// # Errors
    ///
    /// Returns an error if the driver does not support presigning or the
    /// request could not be built.
    pub async fn presign_put_with_policy(
        &self,
        path: &Path,
        expire: Duration,
        options: PresignPutOptions,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<PresignedRequest> {
        strategy.presign_put(self, path, expire, options).await
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

    /// Downloads content from storage as a stream, enabling efficient
    /// handling of large files without loading them entirely into memory.
    ///
    /// This method uses the selected strategy for the download operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// pub async fn stream_download() {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("large_file.mp4");
    ///     
    ///     let stream = storage.download_stream(path).await.unwrap();
    ///     // Stream can be converted to axum Body for HTTP response
    ///     // let body = stream.into_body();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the download operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn download_stream(&self, path: &Path) -> StorageResult<BytesStream> {
        self.download_stream_with_policy(path, &*self.strategy)
            .await
    }

    /// Downloads content from storage as a stream using a specific strategy.
    ///
    /// # Errors
    ///
    /// This method returns an error if the download operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn download_stream_with_policy(
        &self,
        path: &Path,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<BytesStream> {
        strategy.download_stream(self, path).await
    }

    /// Uploads content from a stream to storage, enabling efficient
    /// handling of large files without loading them entirely into memory.
    ///
    /// This method uses the selected strategy for the upload operation.
    ///
    /// # Examples
    ///```
    /// use loco_rs::storage;
    /// use std::path::Path;
    /// pub async fn stream_upload(stream: storage::stream::BytesStream) {
    ///     let storage = storage::Storage::single(storage::drivers::mem::new());
    ///     let path = Path::new("large_file.mp4");
    ///     
    ///     storage.upload_stream(path, stream).await.unwrap();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns an error if the upload operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn upload_stream(&self, path: &Path, stream: BytesStream) -> StorageResult<()> {
        self.upload_stream_with_policy(path, stream, &*self.strategy)
            .await
    }

    /// Uploads content from a stream using a specific strategy.
    ///
    /// # Errors
    ///
    /// This method returns an error if the upload operation fails or if there
    /// is an issue with the strategy configuration.
    pub async fn upload_stream_with_policy(
        &self,
        path: &Path,
        stream: BytesStream,
        strategy: &dyn strategies::StorageStrategy,
    ) -> StorageResult<()> {
        strategy.upload_stream(self, path, stream).await
    }
}

#[cfg(test)]
mod config_provider_tests {
    use super::create_storage_provider;
    use crate::config::StorageConfig;

    /// Absent `storage:` (the `Null` default) must still produce a usable
    /// `Storage` — the same fallback `boot::create_context` hardcoded before
    /// this config field existed.
    #[tokio::test]
    async fn null_config_builds_a_storage_whose_default_store_refuses_reads() {
        let mut config = crate::tests_cfg::config::test_config();
        config.storage = StorageConfig::Null;

        let storage = create_storage_provider(&config).expect("null driver always builds");
        let err = storage
            .exists(std::path::Path::new("anything"))
            .await
            .expect_err("null storage refuses every operation");
        assert!(err.to_string().contains("not supported"));
    }

    /// A `local` config with a path builds a store rooted there, distinct
    /// from `local::new()`'s cwd-rooted default.
    #[tokio::test]
    async fn local_config_with_path_roots_the_store_there() {
        let tree = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("temp dir");

        let mut config = crate::tests_cfg::config::test_config();
        config.storage = StorageConfig::Local(crate::config::LocalStorageConfig {
            path: Some(tree.root.display().to_string()),
        });

        let storage = create_storage_provider(&config).expect("local driver builds");
        let path = std::path::Path::new("probe.txt");
        storage
            .upload(path, &bytes::Bytes::from("loco"))
            .await
            .expect("upload into the configured root");
        assert!(tree.root.join("probe.txt").exists());
    }

    #[cfg(feature = "storage_aws_s3")]
    #[test]
    fn aws_empty_strings_count_as_unset_and_endpoint_alone_is_honored() {
        use crate::config::{AwsStorageConfig, StorageConfig};

        let mut config = crate::tests_cfg::config::test_config();
        for (key, secret, endpoint) in [
            ("", "", ""),
            ("", "", "http://127.0.0.1:9000"),
            ("k", "s", ""),
            ("k", "", ""),
            ("k", "s", "http://127.0.0.1:9000"),
        ] {
            let some = |v: &str| Some(v.to_string());
            config.storage = StorageConfig::Aws(AwsStorageConfig {
                bucket: "b".into(),
                region: "us-east-1".into(),
                endpoint: some(endpoint),
                access_key_id: some(key),
                secret_access_key: some(secret),
                session_token: some(""),
            });
            create_storage_provider(&config)
                .unwrap_or_else(|e| panic!("({key:?},{secret:?},{endpoint:?}) failed: {e}"));
        }
    }
}

#[cfg(test)]
mod multi_storage_provider_tests {
    use std::collections::HashMap;

    use crate::config::{
        LocalStorageConfig, MultiStorageConfig, StorageConfig, StorageStrategyConfig,
    };
    use crate::storage::{strategies::replicated::FailurePolicy, StorageError};

    fn null_stores(strategy: StorageStrategyConfig) -> MultiStorageConfig {
        MultiStorageConfig {
            stores: HashMap::from([
                ("primary".to_string(), StorageConfig::Null),
                ("backup".to_string(), StorageConfig::Null),
            ]),
            strategy,
        }
    }

    #[test]
    fn unknown_default_store_is_rejected() {
        let config = null_stores(StorageStrategyConfig::Single {
            default: "primry".to_string(),
        });

        assert!(matches!(
            super::create_multi_storage_provider(&config),
            Err(StorageError::StoreNotFound(name)) if name == "primry"
        ));
    }

    /// `AllowAll` would otherwise swallow the lookup failure on every write,
    /// so the app would run with no replication and no error.
    #[test]
    fn unknown_secondary_store_is_rejected_even_with_allow_all() {
        let config = null_stores(StorageStrategyConfig::Backup {
            primary: "primary".to_string(),
            secondaries: vec!["bakcup".to_string()],
            failure_policy: FailurePolicy::AllowAll,
        });

        assert!(matches!(
            super::create_multi_storage_provider(&config),
            Err(StorageError::StoreNotFound(name)) if name == "bakcup"
        ));
    }

    /// A store listed twice gets every write twice: the second `rename` finds
    /// the file already moved and fails the whole operation.
    #[test]
    fn store_listed_twice_is_rejected() {
        let config = null_stores(StorageStrategyConfig::Mirror {
            primary: "primary".to_string(),
            secondaries: vec!["primary".to_string()],
            failure_policy: FailurePolicy::FailIfAny,
        });

        assert!(matches!(
            super::create_multi_storage_provider(&config),
            Err(StorageError::Any(_))
        ));
    }

    #[test]
    fn empty_secondaries_is_rejected() {
        let config = null_stores(StorageStrategyConfig::Mirror {
            primary: "primary".to_string(),
            secondaries: vec![],
            failure_policy: FailurePolicy::AllowAll,
        });

        assert!(matches!(
            super::create_multi_storage_provider(&config),
            Err(StorageError::Any(_))
        ));
    }

    /// Parses the way `MultiStorageInitializer` does: YAML into a
    /// `serde_json::Value`, then into the typed config.
    fn parse(yaml: &str) -> MultiStorageConfig {
        let raw: serde_json::Value = serde_yaml::from_str(yaml).expect("valid yaml");
        serde_json::from_value(raw).expect("valid storage config")
    }

    #[test]
    fn mirror_config_parses_from_yaml() {
        let config = parse(
            r#"
            stores:
              primary: { kind: Local, path: storage/primary }
              mirror: { kind: "Null" }
            strategy:
              kind: Mirror
              primary: primary
              secondaries: [mirror]
              failure_policy: { FailAtFailures: 2 }
            "#,
        );

        assert_eq!(config.stores.len(), 2);
        assert!(matches!(
            config.strategy,
            StorageStrategyConfig::Mirror {
                ref primary,
                ref secondaries,
                failure_policy: FailurePolicy::FailAtFailures(2),
            } if primary == "primary" && secondaries == &["mirror"]
        ));
    }

    #[test]
    fn backup_config_parses_from_yaml_with_a_unit_policy_or_none() {
        let with_policy = parse(
            r"
            stores: { primary: { kind: Local } }
            strategy: { kind: Backup, primary: primary, secondaries: [], failure_policy: AllowAll }
            ",
        );
        assert!(matches!(
            with_policy.strategy,
            StorageStrategyConfig::Backup {
                failure_policy: FailurePolicy::AllowAll,
                ..
            }
        ));

        let without_policy = parse(
            r"
            stores: { primary: { kind: Local } }
            strategy: { kind: Backup, primary: primary, secondaries: [] }
            ",
        );
        assert!(matches!(
            without_policy.strategy,
            StorageStrategyConfig::Backup {
                failure_policy: FailurePolicy::FailIfAny,
                ..
            }
        ));
    }

    #[test]
    fn misspelled_secondaries_key_is_a_config_error() {
        let raw = serde_json::json!({
            "stores": { "primary": { "kind": "Local" } },
            "strategy": { "kind": "Mirror", "primary": "primary", "secondries": ["primary"] },
        });

        let err = serde_json::from_value::<MultiStorageConfig>(raw).expect_err("key is misspelled");
        assert!(err.to_string().contains("secondaries"));
    }

    fn local_store(tree: &tree_fs::Tree) -> StorageConfig {
        StorageConfig::Local(LocalStorageConfig {
            path: Some(tree.root.display().to_string()),
        })
    }

    fn temp_dir() -> tree_fs::Tree {
        tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("temp dir")
    }

    /// `Single` strategy: unqualified calls hit `default`; the other named
    /// store is untouched, reachable only by name — the "independent extra
    /// bucket, no replication" shape.
    #[tokio::test]
    async fn single_strategy_routes_unqualified_calls_to_default_only() {
        let primary_tree = temp_dir();
        let other_tree = temp_dir();

        let config = MultiStorageConfig {
            stores: HashMap::from([
                ("primary".to_string(), local_store(&primary_tree)),
                ("other".to_string(), local_store(&other_tree)),
            ]),
            strategy: StorageStrategyConfig::Single {
                default: "primary".to_string(),
            },
        };

        let storage = super::create_multi_storage_provider(&config).expect("builds");
        let path = std::path::Path::new("f.txt");
        storage
            .upload(path, &bytes::Bytes::from("x"))
            .await
            .expect("unqualified upload hits default");

        assert!(primary_tree.root.join("f.txt").exists());
        assert!(!other_tree.root.join("f.txt").exists());

        // "other" is still directly reachable, just outside the strategy.
        storage
            .as_store_err("other")
            .expect("present")
            .upload(path, &bytes::Bytes::from("y"))
            .await
            .expect("direct upload to a non-default store");
        assert!(other_tree.root.join("f.txt").exists());
    }

    /// `Mirror` strategy: an unqualified write fans out to every secondary.
    #[tokio::test]
    async fn mirror_strategy_fans_writes_out_to_secondaries() {
        let primary_tree = temp_dir();
        let mirror_tree = temp_dir();

        let config = MultiStorageConfig {
            stores: HashMap::from([
                ("primary".to_string(), local_store(&primary_tree)),
                ("mirror".to_string(), local_store(&mirror_tree)),
            ]),
            strategy: StorageStrategyConfig::Mirror {
                primary: "primary".to_string(),
                secondaries: vec!["mirror".to_string()],
                failure_policy: FailurePolicy::FailIfAny,
            },
        };

        let storage = super::create_multi_storage_provider(&config).expect("builds");
        let path = std::path::Path::new("f.txt");
        storage
            .upload(path, &bytes::Bytes::from("x"))
            .await
            .expect("mirrored upload");

        assert!(primary_tree.root.join("f.txt").exists());
        assert!(mirror_tree.root.join("f.txt").exists());
    }

    /// `Backup` strategy: writes still fan out, but a download always reads
    /// from `primary` — even after deleting the file there, the read must
    /// fail rather than silently falling back to the secondary.
    #[tokio::test]
    async fn backup_strategy_never_reads_from_a_secondary() {
        let primary_tree = temp_dir();
        let backup_tree = temp_dir();

        let config = MultiStorageConfig {
            stores: HashMap::from([
                ("primary".to_string(), local_store(&primary_tree)),
                ("backup".to_string(), local_store(&backup_tree)),
            ]),
            strategy: StorageStrategyConfig::Backup {
                primary: "primary".to_string(),
                secondaries: vec!["backup".to_string()],
                failure_policy: FailurePolicy::AllowAll,
            },
        };

        let storage = super::create_multi_storage_provider(&config).expect("builds");
        let path = std::path::Path::new("f.txt");
        storage
            .upload(path, &bytes::Bytes::from("x"))
            .await
            .expect("backed-up upload");

        storage
            .as_store_err("primary")
            .expect("present")
            .delete(path)
            .await
            .expect("delete from primary directly");

        let result: crate::storage::StorageResult<String> = storage.download(path).await;
        assert!(
            result.is_err(),
            "backup mode must not fall back to the secondary on read"
        );
    }
}
