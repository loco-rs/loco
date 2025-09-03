//! # `MirrorStrategy` Implementation for Storage Strategies
//!
//! This module provides an implementation of the [`StorageStrategy`] for
//! the [`MirrorStrategy`]. The [`MirrorStrategy`] is designed to mirror storage
//! operations.
//!
//! ## Strategy Description per operation
//!
//! * `upload`/`delete`/`rename`/`copy`: The primary storage must succeed in the
//!   given operation. If there is any failure with the primary storage, this
//!   function returns an error. When
//!   * [`FailureMode::MirrorAll`] is given - all the secondary storages must
//!     succeed. If there is one failure in the mirror, the operation continues
//!     to the rest but returns an error.
//!   * [`FailureMode::AllowMirrorFailure`] is given - the operation does not
//!     return an error when one or more mirror operations fail.
//!
//! * `download`: Initiates the download of the given path from the primary
//!   storage. If successful, it returns the content. If not found in the
//!   primary, it looks for the content in the secondary storages. If the
//!   content is not found in any storage backend (both primary and secondary),
//!   it returns an error.
use std::{collections::BTreeMap, path::Path};

use bytes::Bytes;

use crate::storage::{strategies::StorageStrategy, Storage, StorageError, StorageResult};

/// Enum representing the failure mode for the [`MirrorStrategy`].
#[derive(Clone, Debug)]
pub enum FailureMode {
    /// Fail if any secondary storage mirror encounters an error.
    MirrorAll,
    /// Allow errors from secondary storage mirror without failing.
    AllowMirrorFailure,
}

/// Represents the Mirror Strategy for storage operations.
#[derive(Clone, Debug)]
pub struct MirrorStrategy {
    /// The primary storage backend.
    pub primary: String,
    /// Optional secondary storage backends.
    pub secondaries: Option<Vec<String>>,
    /// The failure mode for handling errors from secondary storage backends.
    pub failure_mode: FailureMode,
}

/// Implementation of the [`StorageStrategy`] for the [`MirrorStrategy`].
///
/// The [`MirrorStrategy`] is designed to mirror operations (upload, download,
/// delete, rename, copy) across multiple storage backends, with optional
/// secondary storage support and customizable failure modes.
#[async_trait::async_trait]
impl StorageStrategy for MirrorStrategy {
    /// Uploads content to the primary and, if configured, secondary storage
    /// mirror.
    ///
    /// # Errors
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

    /// Downloads content from the primary storage backend. If the primary
    /// fails, attempts to download from secondary backends.
    async fn download(&self, storage: &Storage, path: &Path) -> StorageResult<Bytes> {
        let res = Self::try_download(storage, &self.primary, path).await;

        match res {
            Ok(content) => Ok(content),
            Err(error) => {
                if let Some(secondaries) = self.secondaries.as_ref() {
                    for secondary_store in secondaries {
                        if let Ok(content) =
                            Self::try_download(storage, secondary_store, path).await
                        {
                            return Ok(content);
                        }
                    }
                }

                return Err(error);
            }
        }
    }

    /// Deletes content from the primary and, if configured, secondary storage
    /// mirrors.
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
    /// mirrors.
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

        if let Some(secondaries) = self.secondaries.as_ref() {
            let mut collect_errors: BTreeMap<String, String> = BTreeMap::new();
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
                }

                if self.failure_mode.should_fail(&collect_errors) {
                    return Err(StorageError::Multi(collect_errors));
                }
            }
        }

        Ok(())
    }

    /// Copies content from the primary and, if configured, secondary storage
    /// mirrors.
    ///
    /// Returns a [`StorageResult`] indicating success or an error depend of the
    /// [`FailureMode`].
    async fn copy(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()> {
        storage.as_store_err(&self.primary)?.copy(from, to).await?;

        if let Some(secondaries) = self.secondaries.as_ref() {
            let mut collect_errors: BTreeMap<String, String> = BTreeMap::new();
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
                }

                if self.failure_mode.should_fail(&collect_errors) {
                    return Err(StorageError::Multi(collect_errors));
                }
            }
        }

        Ok(())
    }

    /// Downloads content as a stream from the primary storage, or from
    /// secondary storage if primary fails.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] with the stream
    async fn download_stream(
        &self,
        storage: &Storage,
        path: &Path,
    ) -> StorageResult<super::super::stream::BytesStream> {
        // Try primary first
        if let Ok(stream) = storage.as_store_err(&self.primary)?.get_stream(path).await {
            Ok(stream)
        } else {
            // If primary fails, try secondaries
            if let Some(secondaries) = self.secondaries.as_ref() {
                for secondary_store in secondaries {
                    if let Some(store) = storage.as_store(secondary_store) {
                        if let Ok(stream) = store.get_stream(path).await {
                            return Ok(stream);
                        }
                    }
                }
            }
            // If all failed, return error from primary
            storage.as_store_err(&self.primary)?.get_stream(path).await
        }
    }

    /// Uploads content from a stream to the primary and secondary storage
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn upload_stream(
        &self,
        storage: &Storage,
        path: &Path,
        stream: super::super::stream::BytesStream,
    ) -> StorageResult<()> {
        // For mirroring, we need to buffer the stream content once
        // to be able to upload to multiple stores
        let content = stream
            .collect()
            .await
            .map_err(|e| StorageError::Any(Box::new(e)))?;

        // Upload to primary
        storage
            .as_store_err(&self.primary)?
            .upload(path, &content)
            .await?;

        // Upload to secondaries if configured
        if let Some(secondaries) = self.secondaries.as_ref() {
            let mut collect_errors: BTreeMap<String, String> = BTreeMap::new();
            for secondary_store in secondaries {
                match storage.as_store_err(secondary_store) {
                    Ok(store) => {
                        if let Err(err) = store.upload(path, &content).await {
                            collect_errors.insert(secondary_store.to_string(), err.to_string());
                        }
                    }
                    Err(err) => {
                        collect_errors.insert(secondary_store.to_string(), err.to_string());
                    }
                }

                if self.failure_mode.should_fail(&collect_errors) {
                    return Err(StorageError::Multi(collect_errors));
                }
            }
        }

        Ok(())
    }
}

impl MirrorStrategy {
    /// Creates a new instance of [`MirrorStrategy`].
    #[must_use]
    pub fn new(primary: &str, secondaries: Option<Vec<String>>, failure_mode: FailureMode) -> Self {
        Self {
            primary: primary.to_string(),
            secondaries,
            failure_mode,
        }
    }

    // Private helper function for downloading from a specific store.
    async fn try_download(
        storage: &Storage,
        store_name: &str,
        path: &Path,
    ) -> StorageResult<Bytes> {
        let store = storage.as_store_err(store_name)?;
        store.get(path).await?.bytes().await
    }
}

impl FailureMode {
    #[must_use]
    pub fn should_fail(&self, errors: &BTreeMap<String, String>) -> bool {
        match self {
            Self::MirrorAll => !errors.is_empty(),
            Self::AllowMirrorFailure => false,
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{collections::BTreeMap, path::PathBuf};

    use super::*;
    use crate::storage::{drivers, Storage};

    #[tokio::test]
    async fn upload_should_pass_with_mirror_all_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::MirrorAll,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(store_2.exists(path.as_path()).await.unwrap());
        assert!(store_3.exists(path.as_path()).await.unwrap());
    }

    #[cfg(feature = "storage_aws_s3")]
    #[tokio::test]
    async fn upload_should_fail_with_mirror_all_policy() {
        let store_1 = drivers::aws::with_failure();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::MirrorAll,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_err());

        assert!(!store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(!store_3.exists(path.as_path()).await.unwrap());
    }

    #[cfg(feature = "storage_aws_s3")]
    #[tokio::test]
    async fn upload_should_fail_when_allow_mirror_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::aws::with_failure();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AllowMirrorFailure,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(store_3.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn can_download_when_primary_is_ok() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::MirrorAll,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        let content: String = storage.download(path.as_path()).await.unwrap();
        assert_eq!(content, "file content".to_string());

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(store_2.exists(path.as_path()).await.unwrap());
        assert!(store_3.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn can_download_when_primary_failed() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec![
                "store_1".to_string(),
                "store_2".to_string(),
                "store_3".to_string(),
            ]),
            FailureMode::MirrorAll,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store_1.delete(path.as_path()).await.is_ok());
        assert!(store_2.delete(path.as_path()).await.is_ok());

        assert!(!store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(store_3.exists(path.as_path()).await.unwrap());

        let content: String = storage.download(path.as_path()).await.unwrap();
        assert_eq!(content, "file content".to_string());
    }

    #[tokio::test]
    async fn rename_should_pass_when_primary_is_ok() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::MirrorAll,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

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
    async fn rename_should_fail_when_primary_failed() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::MirrorAll,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

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
            .is_err());
    }

    #[tokio::test]
    async fn rename_should_pass_when_allow_mirror_failure() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AllowMirrorFailure,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

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
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_pass_when_primary_is_ok() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::MirrorAll,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

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
    async fn copy_should_pass_fail_when_primary() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::MirrorAll,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

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
            .is_err());
    }

    #[tokio::test]
    async fn should_pass_when_allow_mirror_failure() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(MirrorStrategy::new(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailureMode::AllowMirrorFailure,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
                ("store_3".to_string(), store_3),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();
        let store_3 = storage.as_store("store_3").unwrap();

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
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }
}
