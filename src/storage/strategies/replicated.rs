//! # `ReplicatedStrategy` Implementation for Storage Strategies
//!
//! Replicates storage operations across a primary store and optional secondary
//! stores. This unifies the former `MirrorStrategy` and `BackupStrategy`, which
//! shared the same write-fan-out skeleton and differed only in (a) whether
//! reads fall back to secondaries and (b) how many secondary failures are
//! tolerated.
//!
//! ## Per-operation behavior
//!
//! * `upload`/`delete`/`rename`/`copy`/`upload_stream`: the primary store must
//!   succeed, otherwise the operation returns an error immediately. The same
//!   operation is then fanned out **concurrently** to every secondary; every
//!   secondary is always attempted. Errors are collected per store and the
//!   [`FailurePolicy`] decides whether the overall operation fails.
//! * `download`/`download_stream`: served from the primary. If the primary
//!   fails and `read_from_secondaries` is set (mirror behavior), each secondary
//!   is tried in turn until one succeeds; otherwise (backup behavior) the
//!   primary error is returned.
use std::{collections::BTreeMap, path::Path};

use bytes::Bytes;

use crate::storage::{
    drivers::StoreDriver, strategies::StorageStrategy, Storage, StorageError, StorageResult,
};

/// How many secondary-store failures a [`ReplicatedStrategy`] tolerates before
/// the overall operation is considered failed.
#[derive(Clone, Debug)]
pub enum FailurePolicy {
    /// Fail if any secondary store errors.
    FailIfAny,
    /// Never fail because of secondary-store errors.
    AllowAll,
    /// Tolerate a single secondary failure; fail if more than one errors.
    AllowSingleFailure,
    /// Fail once the number of secondary failures reaches `count`.
    FailAtFailures(usize),
}

impl FailurePolicy {
    #[must_use]
    pub fn should_fail(&self, errors: &BTreeMap<String, String>) -> bool {
        match self {
            Self::FailIfAny => !errors.is_empty(),
            Self::AllowAll => false,
            Self::AllowSingleFailure => errors.len() > 1,
            Self::FailAtFailures(count) => *count <= errors.len(),
        }
    }
}

/// Replicates operations across a primary store and optional secondaries.
#[derive(Clone, Debug)]
pub struct ReplicatedStrategy {
    /// The primary storage backend.
    pub primary: String,
    /// Optional secondary storage backends.
    pub secondaries: Option<Vec<String>>,
    /// Policy deciding when secondary failures fail the overall operation.
    pub failure_policy: FailurePolicy,
    /// When `true`, reads (`download`/`download_stream`) fall back to
    /// secondaries if the primary fails (mirror behavior). When `false`, reads
    /// are served from the primary only (backup behavior).
    pub read_from_secondaries: bool,
}

impl ReplicatedStrategy {
    /// Creates a new [`ReplicatedStrategy`].
    #[must_use]
    pub fn new(
        primary: &str,
        secondaries: Option<Vec<String>>,
        failure_policy: FailurePolicy,
        read_from_secondaries: bool,
    ) -> Self {
        Self {
            primary: primary.to_string(),
            secondaries,
            failure_policy,
            read_from_secondaries,
        }
    }

    /// Mirror configuration: reads fall back to secondaries when the primary
    /// misses.
    #[must_use]
    pub fn mirror(
        primary: &str,
        secondaries: Option<Vec<String>>,
        failure_policy: FailurePolicy,
    ) -> Self {
        Self::new(primary, secondaries, failure_policy, true)
    }

    /// Backup configuration: reads are served from the primary only.
    #[must_use]
    pub fn backup(
        primary: &str,
        secondaries: Option<Vec<String>>,
        failure_policy: FailurePolicy,
    ) -> Self {
        Self::new(primary, secondaries, failure_policy, false)
    }

    // Private helper for downloading (buffered) from a specific store.
    async fn try_download(
        storage: &Storage,
        store_name: &str,
        path: &Path,
    ) -> StorageResult<Bytes> {
        let store = storage.as_store_err(store_name)?;
        store.get(path).await?.bytes().await
    }

    /// Fans `op` out to every secondary store concurrently and collects every
    /// error (including a secondary that fails to resolve via
    /// [`Storage::as_store_err`]) keyed by the secondary store name. Every
    /// secondary is always attempted.
    // The returned future is intentionally not `Send`: the fan-out runs on the
    // current task via `join_all` (never spawned onto another thread), so a
    // `Send` bound would only leak an unnecessary constraint onto every `op`.
    #[allow(clippy::future_not_send)]
    async fn fan_out_to_secondaries<'a, F, Fut, T>(
        &self,
        storage: &'a Storage,
        op: F,
    ) -> BTreeMap<String, String>
    where
        F: Fn(&'a dyn StoreDriver) -> Fut,
        Fut: std::future::Future<Output = StorageResult<T>>,
    {
        let Some(secondaries) = self.secondaries.as_ref() else {
            return BTreeMap::new();
        };

        let op = &op;
        let tasks = secondaries.iter().map(|secondary_store| async move {
            let result = match storage.as_store_err(secondary_store) {
                Ok(store) => op(store).await,
                Err(err) => Err(err),
            };
            result
                .err()
                .map(|err| (secondary_store.clone(), err.to_string()))
        });

        futures_util::future::join_all(tasks)
            .await
            .into_iter()
            .flatten()
            .collect()
    }
}

#[async_trait::async_trait]
impl StorageStrategy for ReplicatedStrategy {
    async fn upload(&self, storage: &Storage, path: &Path, content: &Bytes) -> StorageResult<()> {
        storage
            .as_store_err(&self.primary)?
            .upload(path, content)
            .await?;
        let errors = self
            .fan_out_to_secondaries(storage, |store| store.upload(path, content))
            .await;
        if self.failure_policy.should_fail(&errors) {
            return Err(StorageError::Multi(errors));
        }
        Ok(())
    }

    async fn download(&self, storage: &Storage, path: &Path) -> StorageResult<Bytes> {
        match Self::try_download(storage, &self.primary, path).await {
            Ok(content) => Ok(content),
            Err(error) => {
                if self.read_from_secondaries
                    && let Some(secondaries) = self.secondaries.as_ref()
                {
                    for secondary_store in secondaries {
                        if let Ok(content) =
                            Self::try_download(storage, secondary_store, path).await
                        {
                            return Ok(content);
                        }
                    }
                }
                Err(error)
            }
        }
    }

    async fn delete(&self, storage: &Storage, path: &Path) -> StorageResult<()> {
        storage.as_store_err(&self.primary)?.delete(path).await?;
        let errors = self
            .fan_out_to_secondaries(storage, |store| store.delete(path))
            .await;
        if self.failure_policy.should_fail(&errors) {
            return Err(StorageError::Multi(errors));
        }
        Ok(())
    }

    async fn rename(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()> {
        storage
            .as_store_err(&self.primary)?
            .rename(from, to)
            .await?;
        let errors = self
            .fan_out_to_secondaries(storage, |store| store.rename(from, to))
            .await;
        if self.failure_policy.should_fail(&errors) {
            return Err(StorageError::Multi(errors));
        }
        Ok(())
    }

    async fn copy(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()> {
        storage.as_store_err(&self.primary)?.copy(from, to).await?;
        let errors = self
            .fan_out_to_secondaries(storage, |store| store.copy(from, to))
            .await;
        if self.failure_policy.should_fail(&errors) {
            return Err(StorageError::Multi(errors));
        }
        Ok(())
    }

    async fn download_stream(
        &self,
        storage: &Storage,
        path: &Path,
    ) -> StorageResult<super::super::stream::BytesStream> {
        match storage.as_store_err(&self.primary)?.get_stream(path).await {
            Ok(stream) => Ok(stream),
            Err(error) => {
                if self.read_from_secondaries
                    && let Some(secondaries) = self.secondaries.as_ref()
                {
                    for secondary_store in secondaries {
                        if let Some(store) = storage.as_store(secondary_store)
                            && let Ok(stream) = store.get_stream(path).await
                        {
                            return Ok(stream);
                        }
                    }
                }
                Err(error)
            }
        }
    }

    async fn upload_stream(
        &self,
        storage: &Storage,
        path: &Path,
        stream: super::super::stream::BytesStream,
    ) -> StorageResult<()> {
        let content = stream
            .collect()
            .await
            .map_err(|e| StorageError::Any(Box::new(e)))?;
        storage
            .as_store_err(&self.primary)?
            .upload(path, &content)
            .await?;
        let errors = self
            .fan_out_to_secondaries(storage, |store| store.upload(path, &content))
            .await;
        if self.failure_policy.should_fail(&errors) {
            return Err(StorageError::Multi(errors));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::{collections::BTreeMap, path::PathBuf};

    use super::*;
    use crate::storage::{drivers, Storage};

    // ---------------------------------------------------------------
    // Ported from `mirror.rs`
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn upload_should_pass_with_mirror_all_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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

        let strategy = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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

        let strategy = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowAll,
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

        let strategy = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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

        let strategy = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec![
                "store_1".to_string(),
                "store_2".to_string(),
                "store_3".to_string(),
            ]),
            FailurePolicy::FailIfAny,
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

        let strategy = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowAll,
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

    // Regression test for a short-circuit bug: `rename` used to check
    // `should_fail` *inside* the secondary loop and return as soon as the
    // first secondary errored, leaving any later secondaries un-mirrored.
    // Here the first secondary ("missing_store") does not exist in the
    // `Storage`, so `as_store_err` fails for it immediately. Under the buggy
    // behavior the loop returned before ever attempting "store_2", so
    // "store_2" would still hold stale data at `orig_path` and never receive
    // the rename to `new_path`.
    #[tokio::test]
    async fn rename_attempts_all_secondaries_even_when_first_secondary_fails() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["missing_store".to_string(), "store_2".to_string()]),
            FailurePolicy::FailIfAny,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        // Upload directly to the underlying stores so the missing secondary
        // doesn't cause the setup itself to fail.
        assert!(store_1
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());
        assert!(store_2
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        let result = storage
            .rename(orig_path.as_path(), new_path.as_path())
            .await;
        assert!(result.is_err());

        // "missing_store" failing to resolve is still collected as an error.
        if let Err(StorageError::Multi(errors)) = result {
            assert!(errors.contains_key("missing_store"));
        } else {
            panic!("expected a StorageError::Multi error");
        }

        // The primary always succeeds.
        assert!(!store_1.exists(orig_path.as_path()).await.unwrap());
        assert!(store_1.exists(new_path.as_path()).await.unwrap());

        // The regression: store_2 must still be attempted (and succeed) even
        // though the earlier "missing_store" secondary failed.
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_2.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_pass_when_primary_is_ok() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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

    // Regression test mirroring `rename_attempts_all_secondaries_even_when_
    // first_secondary_fails` for `copy`: the first secondary fails to resolve
    // via `as_store_err`, and the second secondary must still be attempted.
    #[tokio::test]
    async fn copy_attempts_all_secondaries_even_when_first_secondary_fails() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["missing_store".to_string(), "store_2".to_string()]),
            FailurePolicy::FailIfAny,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();

        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let new_path = PathBuf::from("data-2").join("data").join("2.txt");
        let file_content = Bytes::from("file content");

        assert!(store_1
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());
        assert!(store_2
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        let result = storage.copy(orig_path.as_path(), new_path.as_path()).await;
        assert!(result.is_err());

        if let Err(StorageError::Multi(errors)) = result {
            assert!(errors.contains_key("missing_store"));
        } else {
            panic!("expected a StorageError::Multi error");
        }

        // The regression: store_2 must still receive the copy even though
        // the earlier "missing_store" secondary failed.
        assert!(store_2.exists(new_path.as_path()).await.unwrap());
    }

    // Regression test mirroring the rename/copy ones, but for `upload_stream`,
    // which had the same in-loop short-circuit bug. The payload is buffered
    // into `Bytes` before fan-out, so every secondary can be attempted. The
    // first secondary ("missing_store") fails to resolve; "store_2" must still
    // receive the streamed upload.
    #[tokio::test]
    async fn upload_stream_attempts_all_secondaries_even_when_first_secondary_fails() {
        use crate::storage::stream::BytesStream;

        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["missing_store".to_string(), "store_2".to_string()]),
            FailurePolicy::FailIfAny,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(
            BTreeMap::from([
                ("store_1".to_string(), store_1),
                ("store_2".to_string(), store_2),
            ]),
            strategy,
        );
        let store_1 = storage.as_store("store_1").unwrap();
        let store_2 = storage.as_store("store_2").unwrap();

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        let stream = BytesStream::from_body_stream(futures_util::stream::once({
            let file_content = file_content.clone();
            async move { Ok(file_content) }
        }));

        let result = storage.upload_stream(path.as_path(), stream).await;
        assert!(result.is_err());

        if let Err(StorageError::Multi(errors)) = result {
            assert!(errors.contains_key("missing_store"));
        } else {
            panic!("expected a StorageError::Multi error");
        }

        // The primary always succeeds.
        assert!(store_1.exists(path.as_path()).await.unwrap());

        // The regression: store_2 must still receive the streamed upload even
        // though the earlier "missing_store" secondary failed.
        assert!(store_2.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn should_pass_when_allow_mirror_failure() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowAll,
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

    // ---------------------------------------------------------------
    // Ported from `backup.rs`
    // ---------------------------------------------------------------

    // Upload

    #[tokio::test]
    async fn upload_should_pass_when_backup_all_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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
    async fn upload_should_fail_when_primary_fail() {
        let store_1 = drivers::aws::with_failure();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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
    async fn upload_should_pass_when_allow_backup_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::aws::with_failure();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowAll,
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

    #[cfg(feature = "storage_aws_s3")]
    #[tokio::test]
    async fn upload_should_pass_when_at_least_one_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::aws::with_failure();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowSingleFailure,
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

    #[cfg(feature = "storage_aws_s3")]
    #[tokio::test]
    async fn upload_should_fail_when_at_least_one_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::aws::with_failure();
        let store_3 = drivers::aws::with_failure();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailAtFailures(2),
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

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(!store_3.exists(path.as_path()).await.unwrap());
    }

    #[cfg(feature = "storage_aws_s3")]
    #[tokio::test]
    async fn upload_should_pass_count_fail_policy_should_pass() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::aws::with_failure();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailAtFailures(2),
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

    #[cfg(feature = "storage_aws_s3")]
    #[tokio::test]
    async fn upload_should_fail_when_count_fail_should_fail() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::aws::with_failure();
        let store_3 = drivers::aws::with_failure();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailAtFailures(2),
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

        assert!(store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(!store_3.exists(path.as_path()).await.unwrap());
    }

    // Download

    #[tokio::test]
    async fn can_download() {
        let store_1 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
        )) as Box<dyn StorageStrategy>;

        let storage = Storage::new(BTreeMap::from([("store_1".to_string(), store_1)]), strategy);
        let store_1 = storage.as_store("store_1").unwrap();

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
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowAll,
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

        assert!(storage.delete(path.as_path()).await.is_ok());

        assert!(!store_1.exists(path.as_path()).await.unwrap());
        assert!(!store_2.exists(path.as_path()).await.unwrap());
        assert!(!store_3.exists(path.as_path()).await.unwrap());
    }

    // rename
    #[tokio::test]
    async fn rename_should_pass_when_backup_all_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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
    async fn rename_should_pass_when_allow_backup_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowAll,
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
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn rename_should_pass_when_at_least_one_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowSingleFailure,
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
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn rename_should_fail_when_at_least_one_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowSingleFailure,
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
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailAtFailures(2),
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
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(!store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn rename_should_fail_when_count_fail_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailAtFailures(2),
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
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailIfAny,
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
    async fn copy_should_pass_when_allow_backup_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowAll,
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
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_pass_when_at_least_one_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowSingleFailure,
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
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_fail_when_at_least_one_failure_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::AllowSingleFailure,
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
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailAtFailures(2),
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
        assert!(!store_2.exists(orig_path.as_path()).await.unwrap());
        assert!(store_3.exists(orig_path.as_path()).await.unwrap());

        assert!(store_1.exists(new_path.as_path()).await.unwrap());
        assert!(!store_2.exists(new_path.as_path()).await.unwrap());
        assert!(store_3.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn copy_should_fail_when_count_fail_policy() {
        let store_1 = drivers::mem::new();
        let store_2 = drivers::mem::new();
        let store_3 = drivers::mem::new();

        let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
            "store_1",
            Some(vec!["store_2".to_string(), "store_3".to_string()]),
            FailurePolicy::FailAtFailures(2),
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
