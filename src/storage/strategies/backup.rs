//! # `BackupStrategy` Implementation for Storage Strategies
//!
//! This module provides an implementation of the [`StorageStrategyTrait`] for
//! the [`BackupStrategy`]. The [`BackupStrategy`] is designed to mirror storage
//! operations.
//!
//! ## Strategy Description per operation
//!
//! * `upload`/`delete`/`rename`/`copy`: The primary storage must succeed in the
//!   given operation. If there is any failure with the primary storage, this
//!   function returns an error. When
//!   * [`FailureMode::BackupAll`] is given - all the secondary storages must
//!     succeed. If there is one failure in the backup, the operation continues
//!     to the rest but returns an error.
//!   * [`FailureMode::AllowBackupFailure`] is given - the operation does not
//!     return an error when one or more mirror operations fail.
//!   * [`FailureMode::AtLeastOneFailure`] is given - at least one operation
//!     should pass.
//!   * [`FailureMode::CountFailure`] is given - the number of the given backup
//!     should pass.
//!
//! * `download`: Initiates the download of the given path only from primary
//!   storage.
use std::{collections::BTreeMap, path::Path};

use bytes::Bytes;

use crate::storage::{
    error::{StorageError, StorageResult, StoreError},
    strategies::StorageStrategyTrait,
    Storage,
};

/// Enum representing the failure mode for the [`BackupStrategy`].
#[derive(Clone)]
pub enum FailureMode {
    /// Fail if any secondary storage backend encounters an error.
    BackupAll,
    /// Allow errors from secondary storage backup without failing.
    AllowBackupFailure,
    /// Allow only one backup failure from secondary storage backup without
    /// failing.
    AtLeastOneFailure,
    /// Allow the given backup number to failure from secondary storage backup
    /// without failing.
    CountFailure(usize),
}

/// Represents the Backup Strategy for storage operations.
#[derive(Clone)]
pub struct BackupStrategy {
    pub primary: String,
    pub secondaries: Option<Vec<String>>,
    pub failure_mode: FailureMode,
}

#[async_trait::async_trait]
impl StorageStrategyTrait for BackupStrategy {
    /// Uploads content to the primary and, if configured, secondary storage
    /// backends.
    // # Errors
    ///
    /// Returns a [`StorageResult`] indicating success or an error depend of the
    /// [`FailureMode`].
    async fn upload(&self, storage: &Storage, path: &Path, content: &Bytes) -> StorageResult<()> {
        storage
            .as_store_err(&self.primary)?
            .upload(path, content)
            .await?;

        let mut collect_errors: BTreeMap<String, String> = BTreeMap::new();
        if let Some(secondaries) = self.secondaries.as_ref() {
            for secondary_store in secondaries {
                match storage.as_store_err(secondary_store) {
                    Ok(store) => {
                        if let Err(err) = store.upload(path, content).await {
                            collect_errors.insert(secondary_store.to_string(), err.to_string());
                        }
                    }
                    Err(err) => {
                        collect_errors.insert(secondary_store.to_string(), err.to_string());
                    }
                };
            }
        }

        if self.failure_mode.should_fail(&collect_errors) {
            return Err(StorageError::Multi(collect_errors));
        }

        Ok(())
    }

    /// Downloads content only from primary storage backend.
    async fn download(&self, storage: &Storage, path: &Path) -> StorageResult<Bytes> {
        let store = storage.as_store_err(&self.primary)?;
        Ok(store
            .get(path)
            .await?
            .bytes()
            .await
            .map_err(|e| StorageError::Storage(StoreError::Storage(e)))?)
    }

    /// Deletes content from the primary and, if configured, secondary storage
    /// backends.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating success or an error depend of the
    /// [`FailureMode`].
    async fn delete(&self, storage: &Storage, path: &Path) -> StorageResult<()> {
        storage.as_store_err(&self.primary)?.delete(path).await?;

        let mut collect_errors: BTreeMap<String, String> = BTreeMap::new();
        if let Some(secondaries) = self.secondaries.as_ref() {
            for secondary_store in secondaries {
                match storage.as_store_err(secondary_store) {
                    Ok(store) => {
                        if let Err(err) = store.delete(path).await {
                            collect_errors.insert(secondary_store.to_string(), err.to_string());
                        }
                    }
                    Err(err) => {
                        collect_errors.insert(secondary_store.to_string(), err.to_string());
                    }
                };
            }
        }

        if self.failure_mode.should_fail(&collect_errors) {
            return Err(StorageError::Multi(collect_errors));
        }

        Ok(())
    }

    /// Renames content on the primary and, if configured, secondary storage
    /// backends.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating success or an error depend of the
    /// [`FailureMode`].
    async fn rename(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()> {
        storage
            .as_store_err(&self.primary)?
            .rename(from, to)
            .await?;

        let mut collect_errors: BTreeMap<String, String> = BTreeMap::new();
        if let Some(secondaries) = self.secondaries.as_ref() {
            for secondary_store in secondaries {
                match storage.as_store_err(secondary_store) {
                    Ok(store) => {
                        if let Err(err) = store.rename(from, to).await {
                            collect_errors.insert(secondary_store.to_string(), err.to_string());
                        }
                    }
                    Err(err) => {
                        collect_errors.insert(secondary_store.to_string(), err.to_string());
                    }
                };
            }
        }

        if self.failure_mode.should_fail(&collect_errors) {
            return Err(StorageError::Multi(collect_errors));
        }

        Ok(())
    }

    /// Copies content from the primary and, if configured, secondary storage
    /// backends.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating success or an error depend of the
    /// [`FailureMode`].
    async fn copy(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()> {
        storage.as_store_err(&self.primary)?.copy(from, to).await?;

        let mut collect_errors: BTreeMap<String, String> = BTreeMap::new();
        if let Some(secondaries) = self.secondaries.as_ref() {
            for secondary_store in secondaries {
                match storage.as_store_err(secondary_store) {
                    Ok(store) => {
                        if let Err(err) = store.copy(from, to).await {
                            collect_errors.insert(secondary_store.to_string(), err.to_string());
                        }
                    }
                    Err(err) => {
                        collect_errors.insert(secondary_store.to_string(), err.to_string());
                    }
                };
            }
        }

        if self.failure_mode.should_fail(&collect_errors) {
            return Err(StorageError::Multi(collect_errors));
        }

        Ok(())
    }
}

impl BackupStrategy {
    /// Creates a new instance of [`BackupStrategy`].
    #[must_use]
    pub fn new(primary: &str, secondaries: Option<Vec<String>>, failure_mode: FailureMode) -> Self {
        Self {
            primary: primary.to_string(),
            secondaries,
            failure_mode,
        }
    }
}

impl FailureMode {
    #[must_use]
    pub fn should_fail(&self, errors: &BTreeMap<String, String>) -> bool {
        match self {
            Self::BackupAll => !errors.is_empty(),
            Self::AllowBackupFailure => false,
            Self::AtLeastOneFailure => errors.len() > 1,
            Self::CountFailure(count) => count <= &errors.len(),
        }
    }
}

#[cfg(test)]
mod tests {

    use core::time::Duration;
    use std::{collections::BTreeMap, path::PathBuf};

    use object_store::{aws::AmazonS3Builder, memory::InMemory, BackoffConfig, RetryConfig};

    use super::*;
    use crate::storage::{driver, Storage};

    // Upload

    #[tokio::test]
    async fn upload_should_pass_when_backup_all_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::BackupAll,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(store_2.exists(path.as_path()).await.unwrap());
        assert!(store_3.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn upload_should_fail_when_primary_fail() {
        let store_1 = driver::new(
            (Box::new(
                AmazonS3Builder::new()
                    .with_bucket_name("loco-test")
                    .with_retry(RetryConfig {
                        backoff: BackoffConfig::default(),
                        max_retries: 0,
                        retry_timeout: Duration::from_secs(0),
                    })
                    .build()
                    .unwrap(),
            ) as Box<dyn object_store::ObjectStore>)
                .into(),
        );

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::BackupAll,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_err());

        assert!(!store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(!store_3.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn upload_should_pass_when_allow_backup_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 = driver::new(
            (Box::new(
                AmazonS3Builder::new()
                    .with_bucket_name("loco-test")
                    .with_retry(RetryConfig {
                        backoff: BackoffConfig::default(),
                        max_retries: 0,
                        retry_timeout: Duration::from_secs(0),
                    })
                    .build()
                    .unwrap(),
            ) as Box<dyn object_store::ObjectStore>)
                .into(),
        );

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AllowBackupFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(store_3.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn upload_should_pass_when_at_least_one_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 = driver::new(
            (Box::new(
                AmazonS3Builder::new()
                    .with_bucket_name("loco-test")
                    .with_retry(RetryConfig {
                        backoff: BackoffConfig::default(),
                        max_retries: 0,
                        retry_timeout: Duration::from_secs(0),
                    })
                    .build()
                    .unwrap(),
            ) as Box<dyn object_store::ObjectStore>)
                .into(),
        );

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AtLeastOneFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(store_3.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn upload_should_fail_when_at_least_one_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 = driver::new(
            (Box::new(
                AmazonS3Builder::new()
                    .with_bucket_name("loco-test")
                    .with_retry(RetryConfig {
                        backoff: BackoffConfig::default(),
                        max_retries: 0,
                        retry_timeout: Duration::from_secs(0),
                    })
                    .build()
                    .unwrap(),
            ) as Box<dyn object_store::ObjectStore>)
                .into(),
        );

        let store_3 = driver::new(
            (Box::new(
                AmazonS3Builder::new()
                    .with_bucket_name("loco-test")
                    .with_retry(RetryConfig {
                        backoff: BackoffConfig::default(),
                        max_retries: 0,
                        retry_timeout: Duration::from_secs(0),
                    })
                    .build()
                    .unwrap(),
            ) as Box<dyn object_store::ObjectStore>)
                .into(),
        );

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::CountFailure(2),
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_err());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(!store_3.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn upload_should_pass_count_fail_policy_should_pass() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 = driver::new(
            (Box::new(
                AmazonS3Builder::new()
                    .with_bucket_name("loco-test")
                    .with_retry(RetryConfig {
                        backoff: BackoffConfig::default(),
                        max_retries: 0,
                        retry_timeout: Duration::from_secs(0),
                    })
                    .build()
                    .unwrap(),
            ) as Box<dyn object_store::ObjectStore>)
                .into(),
        );

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::CountFailure(2),
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(store_3.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn upload_should_fail_when_count_fail_should_fail() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 = driver::new(
            (Box::new(
                AmazonS3Builder::new()
                    .with_bucket_name("loco-test")
                    .with_retry(RetryConfig {
                        backoff: BackoffConfig::default(),
                        max_retries: 0,
                        retry_timeout: Duration::from_secs(0),
                    })
                    .build()
                    .unwrap(),
            ) as Box<dyn object_store::ObjectStore>)
                .into(),
        );

        let store_3 = driver::new(
            (Box::new(
                AmazonS3Builder::new()
                    .with_bucket_name("loco-test")
                    .with_retry(RetryConfig {
                        backoff: BackoffConfig::default(),
                        max_retries: 0,
                        retry_timeout: Duration::from_secs(0),
                    })
                    .build()
                    .unwrap(),
            ) as Box<dyn object_store::ObjectStore>)
                .into(),
        );

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::CountFailure(2),
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_err());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(!store_3.exists(path.as_path()).await.unwrap());
    }

    // Download

    #[tokio::test]
    async fn can_download() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::BackupAll,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([("store_1".to_string(), store_1.clone())]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_err());

        let download_file: String = storage.download(path.as_path()).await.unwrap();
        assert_eq!(download_file, file_content);

        assert!(store_1.delete(path.as_path()).await.is_ok());

        let download_file: StorageResult<String> = storage.download(path.as_path()).await;
        assert!(download_file.is_err());
    }

    // Delete

    #[tokio::test]
    async fn delete_should_pass_when_backup_all_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AllowBackupFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(storage.delete(path.as_path()).await.is_ok());

        assert!(!store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(!store_3.exists(path.as_path()).await.unwrap());
    }

    // rename
    #[tokio::test]
    async fn rename_should_pass_when_backup_all_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::BackupAll,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(storage
            .rename(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(!store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn rename_should_pass_when_allow_backup_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AllowBackupFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .rename(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(!store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn rename_should_pass_when_at_least_one_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AtLeastOneFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .rename(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(!store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn rename_should_fail_when_at_least_one_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AtLeastOneFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());
        assert!(store_3.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .rename(orig_path.as_path(), new_path.as_path())
            .await
            .is_err());

        assert!(!store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(!store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn rename_should_pass_when_count_fail_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::CountFailure(2),
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .rename(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(!store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn rename_should_fail_when_count_fail_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::CountFailure(2),
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());
        assert!(store_3.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .rename(orig_path.as_path(), new_path.as_path())
            .await
            .is_err());

        assert!(!store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(!store_3.exists(new_path.as_path()).await.unwrap());
    }

    // Copy

    #[tokio::test]
    async fn copy_should_pass_when_backup_all_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::BackupAll,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(storage
            .copy(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_pass_when_allow_backup_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AllowBackupFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .copy(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_pass_when_at_least_one_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AtLeastOneFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .copy(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_fail_when_at_least_one_failure_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AtLeastOneFailure,
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());
        assert!(store_3.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .copy(orig_path.as_path(), new_path.as_path())
            .await
            .is_err());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(!store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_pass_when_count_fail_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::CountFailure(2),
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .copy(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_fail_when_count_fail_policy() {
        let store_1 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_2 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let store_3 =
            driver::new((Box::new(InMemory::new()) as Box<dyn object_store::ObjectStore>).into());

        let strategy: Box<dyn StorageStrategyTrait> = Box::new(BackupStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::CountFailure(2),
        )) as Box<dyn StorageStrategyTrait>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1.clone()),
                ("store_2".to_string(), store_2.clone()),
                ("store_3".to_string(), store_3.clone()),
            ]),
            strategy.into(),
        );

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_2.delete(orig_path.as_path()).await.is_ok());
        assert!(store_3.delete(orig_path.as_path()).await.is_ok());

        assert!(storage
            .copy(orig_path.as_path(), new_path.as_path())
            .await
            .is_err());

        assert!(store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(!store_3.exists(new_path.as_path()).await.unwrap());
    }
}
