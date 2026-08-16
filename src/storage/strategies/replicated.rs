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
//! * `download`/`download_stream`/`stat`: served from the primary. If the
//!   primary fails and `read_from_secondaries` is set (mirror behavior), each
//!   secondary is tried in turn until one succeeds; otherwise (backup
//!   behavior) the primary error is returned.
//! * `exists`: like the reads above, but a primary `Ok(false)` (missing key)
//!   also triggers secondary fallback under mirror mode, since a miss is not
//!   an error.
//! * `list`: like `exists` — primary `Err` **or** primary `Ok([])` triggers
//!   secondary fallback under mirror mode. Empty secondary listings are
//!   skipped (same as `exists` skipping `false`) so a later secondary with
//!   data remains discoverable.
use std::{collections::BTreeMap, path::Path};

use bytes::Bytes;

use crate::storage::{
    drivers::{ListEntry, StoreDriver},
    strategies::StorageStrategy,
    Storage, StorageError, StorageResult,
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
    /// When `true`, reads (`download`/`download_stream`/`exists`/`list`/`stat`)
    /// fall back to secondaries if the primary fails or, for `exists`/`list`,
    /// reports a miss (mirror behavior). When `false`, reads are served from
    /// the primary only (backup behavior).
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

    // Mirror fallback: `exists`/`list` treat a miss (`false` / `[]`) like an
    // error trigger. Store-resolution errors on the primary also fall back.
    async fn exists(&self, storage: &Storage, path: &Path) -> StorageResult<bool> {
        let primary_result = match storage.as_store_err(&self.primary) {
            Ok(store) => store.exists(path).await,
            Err(err) => Err(err),
        };

        if matches!(&primary_result, Ok(true)) {
            return Ok(true);
        }

        if self.read_from_secondaries
            && matches!(&primary_result, Ok(false) | Err(_))
            && let Some(secondaries) = self.secondaries.as_ref()
        {
            for secondary_store in secondaries {
                if let Some(store) = storage.as_store(secondary_store)
                    && matches!(store.exists(path).await, Ok(true))
                {
                    return Ok(true);
                }
            }
        }

        primary_result
    }

    async fn list(
        &self,
        storage: &Storage,
        path: &Path,
        recursive: bool,
    ) -> StorageResult<Vec<ListEntry>> {
        let primary_result = match storage.as_store_err(&self.primary) {
            Ok(store) => store.list(path, recursive).await,
            Err(err) => Err(err),
        };

        let should_fallback =
            self.read_from_secondaries && primary_result.as_ref().map_or(true, Vec::is_empty);

        if should_fallback && let Some(secondaries) = self.secondaries.as_ref() {
            // Skip empty listings (same as `exists` skipping `false`) so a
            // barren secondary does not hide data on a later one. No hit →
            // return `primary_result` (preserves primary `Err`).
            for secondary_store in secondaries {
                if let Some(store) = storage.as_store(secondary_store)
                    && let Ok(entries) = store.list(path, recursive).await
                    && !entries.is_empty()
                {
                    return Ok(entries);
                }
            }
        }

        primary_result
    }

    async fn stat(&self, storage: &Storage, path: &Path) -> StorageResult<ListEntry> {
        let primary_result = match storage.as_store_err(&self.primary) {
            Ok(store) => store.stat(path).await,
            Err(err) => Err(err),
        };

        match primary_result {
            Ok(entry) => Ok(entry),
            Err(error) => {
                if self.read_from_secondaries
                    && let Some(secondaries) = self.secondaries.as_ref()
                {
                    for secondary_store in secondaries {
                        if let Some(store) = storage.as_store(secondary_store)
                            && let Ok(entry) = store.stat(path).await
                        {
                            return Ok(entry);
                        }
                    }
                }
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
    };

    use async_trait::async_trait;
    use rstest::rstest;

    use super::{
        FailurePolicy::{AllowAll, AllowSingleFailure, FailAtFailures, FailIfAny},
        *,
    };
    use crate::storage::{
        drivers::{self, GetResponse, UploadResponse},
        stream::BytesStream,
        Storage,
    };

    /// A store that refuses every write but answers reads honestly, so a case
    /// can assert "nothing landed here" without the assertion's own read
    /// failing.
    ///
    /// This used to be a real S3 client aimed at a nonexistent bucket, which
    /// made these unit tests do network I/O and confined them to
    /// `--features storage_aws_s3`.
    struct Unwritable;

    fn refused() -> StorageError {
        StorageError::Any("unwritable store".into())
    }

    #[async_trait]
    impl StoreDriver for Unwritable {
        async fn upload(&self, _path: &Path, _content: &Bytes) -> StorageResult<UploadResponse> {
            Err(refused())
        }
        async fn get(&self, _path: &Path) -> StorageResult<GetResponse> {
            Err(refused())
        }
        async fn delete(&self, _path: &Path) -> StorageResult<()> {
            Err(refused())
        }
        async fn rename(&self, _from: &Path, _to: &Path) -> StorageResult<()> {
            Err(refused())
        }
        async fn copy(&self, _from: &Path, _to: &Path) -> StorageResult<()> {
            Err(refused())
        }
        async fn exists(&self, _path: &Path) -> StorageResult<bool> {
            Ok(false)
        }
        async fn list(&self, _path: &Path, _recursive: bool) -> StorageResult<Vec<ListEntry>> {
            Ok(vec![])
        }
        async fn stat(&self, _path: &Path) -> StorageResult<ListEntry> {
            Err(refused())
        }
    }

    const SECONDARIES: [&str; 2] = ["store_2", "store_3"];

    fn content() -> Bytes {
        Bytes::from("file content")
    }

    fn orig() -> PathBuf {
        PathBuf::from("users").join("data").join("1.txt")
    }

    fn dest() -> PathBuf {
        PathBuf::from("data-2").join("data").join("2.txt")
    }

    fn storage_for(
        strategy: ReplicatedStrategy,
        stores: BTreeMap<String, Box<dyn StoreDriver>>,
    ) -> Storage {
        Storage::new(stores, Box::new(strategy) as Box<dyn StorageStrategy>)
    }

    fn named(names: &[&str]) -> BTreeMap<String, Box<dyn StoreDriver>> {
        names
            .iter()
            .map(|n| ((*n).to_string(), drivers::mem::new()))
            .collect()
    }

    /// `store_1`, `store_2`, `store_3`, where a `false` entry is
    /// [`Unwritable`] instead of an in-memory store.
    fn three(writable: [bool; 3]) -> BTreeMap<String, Box<dyn StoreDriver>> {
        writable
            .iter()
            .enumerate()
            .map(|(i, &ok)| {
                let store: Box<dyn StoreDriver> = if ok {
                    drivers::mem::new()
                } else {
                    Box::new(Unwritable)
                };
                (format!("store_{}", i + 1), store)
            })
            .collect()
    }

    /// `store_1` primary, `store_2`/`store_3` secondary. Mirror and backup
    /// differ only in read fallback, so every write case below is run against
    /// both to pin that they stay identical on the write path.
    fn strategy(mirror: bool, policy: FailurePolicy) -> ReplicatedStrategy {
        let secondaries = Some(SECONDARIES.iter().map(|s| (*s).to_string()).collect());
        if mirror {
            ReplicatedStrategy::mirror("store_1", secondaries, policy)
        } else {
            ReplicatedStrategy::backup("store_1", secondaries, policy)
        }
    }

    async fn put(storage: &Storage, store: &str, path: &Path) {
        storage
            .as_store(store)
            .unwrap()
            .upload(path, &content())
            .await
            .unwrap();
    }

    async fn holds(storage: &Storage, store: &str, path: &Path) -> bool {
        storage.as_store(store).unwrap().exists(path).await.unwrap()
    }

    fn paths_of(entries: &[ListEntry]) -> Vec<String> {
        entries.iter().map(|e| e.path.clone()).collect()
    }

    // ---------------------------------------------------------------
    // write fan-out: primary must succeed, policy judges the secondaries
    // ---------------------------------------------------------------

    #[rstest]
    #[case::all_writable([true, true, true], FailIfAny, true, [true, true, true])]
    #[case::primary_unwritable([false, true, true], FailIfAny, false, [false, false, false])]
    #[case::secondary_unwritable([true, false, true], FailIfAny, false, [true, false, true])]
    #[case::secondary_unwritable_allowed([true, false, true], AllowAll, true, [true, false, true])]
    #[case::one_failure_tolerated([true, false, true], AllowSingleFailure, true, [true, false, true])]
    #[case::two_failures_not_tolerated([true, false, false], AllowSingleFailure, false, [true, false, false])]
    #[case::below_failure_count([true, false, true], FailAtFailures(2), true, [true, false, true])]
    #[case::at_failure_count([true, false, false], FailAtFailures(2), false, [true, false, false])]
    #[tokio::test]
    async fn upload(
        #[values(true, false)] mirror: bool,
        #[case] writable: [bool; 3],
        #[case] policy: FailurePolicy,
        #[case] expect_ok: bool,
        #[case] landed: [bool; 3],
    ) {
        let storage = storage_for(strategy(mirror, policy), three(writable));

        assert_eq!(
            storage.upload(&orig(), &content()).await.is_ok(),
            expect_ok,
            "unexpected overall result"
        );

        for (i, expected) in landed.iter().enumerate() {
            let store = format!("store_{}", i + 1);
            assert_eq!(
                holds(&storage, &store, &orig()).await,
                *expected,
                "unexpected content in {store}"
            );
        }
    }

    /// A secondary that no longer holds the source object fails the operation
    /// on that store; the policy then decides the overall result. Every
    /// secondary is attempted regardless.
    #[rstest]
    #[case::nothing_missing(FailIfAny, &[], true)]
    #[case::one_missing(FailIfAny, &["store_2"], false)]
    #[case::one_missing_allowed(AllowAll, &["store_2"], true)]
    #[case::one_missing_tolerated(AllowSingleFailure, &["store_2"], true)]
    #[case::two_missing_not_tolerated(AllowSingleFailure, &["store_2", "store_3"], false)]
    #[case::below_failure_count(FailAtFailures(2), &["store_2"], true)]
    #[case::at_failure_count(FailAtFailures(2), &["store_2", "store_3"], false)]
    #[tokio::test]
    async fn rename(
        #[values(true, false)] mirror: bool,
        #[case] policy: FailurePolicy,
        #[case] missing_from: &[&str],
        #[case] expect_ok: bool,
    ) {
        let storage = storage_for(strategy(mirror, policy), three([true; 3]));
        storage.upload(&orig(), &content()).await.unwrap();
        for store in missing_from {
            storage
                .as_store(store)
                .unwrap()
                .delete(&orig())
                .await
                .unwrap();
        }

        assert_eq!(storage.rename(&orig(), &dest()).await.is_ok(), expect_ok);

        assert!(!holds(&storage, "store_1", &orig()).await);
        assert!(holds(&storage, "store_1", &dest()).await);
        for store in SECONDARIES {
            let had_source = !missing_from.contains(&store);
            assert!(!holds(&storage, store, &orig()).await);
            assert_eq!(
                holds(&storage, store, &dest()).await,
                had_source,
                "{store} should have been renamed too"
            );
        }
    }

    /// Same fan-out as [`rename`], except the source survives.
    #[rstest]
    #[case::nothing_missing(FailIfAny, &[], true)]
    #[case::one_missing(FailIfAny, &["store_2"], false)]
    #[case::one_missing_allowed(AllowAll, &["store_2"], true)]
    #[case::one_missing_tolerated(AllowSingleFailure, &["store_2"], true)]
    #[case::two_missing_not_tolerated(AllowSingleFailure, &["store_2", "store_3"], false)]
    #[case::below_failure_count(FailAtFailures(2), &["store_2"], true)]
    #[case::at_failure_count(FailAtFailures(2), &["store_2", "store_3"], false)]
    #[tokio::test]
    async fn copy(
        #[values(true, false)] mirror: bool,
        #[case] policy: FailurePolicy,
        #[case] missing_from: &[&str],
        #[case] expect_ok: bool,
    ) {
        let storage = storage_for(strategy(mirror, policy), three([true; 3]));
        storage.upload(&orig(), &content()).await.unwrap();
        for store in missing_from {
            storage
                .as_store(store)
                .unwrap()
                .delete(&orig())
                .await
                .unwrap();
        }

        assert_eq!(storage.copy(&orig(), &dest()).await.is_ok(), expect_ok);

        assert!(holds(&storage, "store_1", &orig()).await);
        assert!(holds(&storage, "store_1", &dest()).await);
        for store in SECONDARIES {
            let had_source = !missing_from.contains(&store);
            assert_eq!(holds(&storage, store, &orig()).await, had_source);
            assert_eq!(
                holds(&storage, store, &dest()).await,
                had_source,
                "{store} should have been copied to as well"
            );
        }
    }

    #[rstest]
    #[tokio::test]
    async fn delete(#[values(true, false)] mirror: bool) {
        let storage = storage_for(strategy(mirror, AllowAll), three([true; 3]));
        storage.upload(&orig(), &content()).await.unwrap();

        assert!(storage.delete(&orig()).await.is_ok());

        for store in ["store_1", "store_2", "store_3"] {
            assert!(!holds(&storage, store, &orig()).await);
        }
    }

    // ---------------------------------------------------------------
    // every secondary is attempted even after an earlier one fails
    // ---------------------------------------------------------------

    /// Regression: these operations used to check `should_fail` *inside* the
    /// secondary loop and return as soon as the first secondary errored,
    /// leaving later secondaries un-replicated. `missing_store` is absent from
    /// the `Storage`, so it fails to resolve immediately — `store_2` must still
    /// be reached.
    fn with_missing_first_secondary() -> Storage {
        storage_for(
            ReplicatedStrategy::mirror(
                "store_1",
                Some(vec!["missing_store".to_string(), "store_2".to_string()]),
                FailIfAny,
            ),
            named(&["store_1", "store_2"]),
        )
    }

    fn assert_blames_missing_store(result: &StorageResult<()>) {
        match result {
            Err(StorageError::Multi(errors)) => {
                assert!(errors.contains_key("missing_store"));
            }
            _ => panic!("expected a StorageError::Multi error"),
        }
    }

    #[tokio::test]
    async fn rename_attempts_all_secondaries_even_when_first_secondary_fails() {
        let storage = with_missing_first_secondary();
        put(&storage, "store_1", &orig()).await;
        put(&storage, "store_2", &orig()).await;

        let result = storage.rename(&orig(), &dest()).await;
        assert_blames_missing_store(&result);

        for store in ["store_1", "store_2"] {
            assert!(!holds(&storage, store, &orig()).await);
            assert!(holds(&storage, store, &dest()).await);
        }
    }

    #[tokio::test]
    async fn copy_attempts_all_secondaries_even_when_first_secondary_fails() {
        let storage = with_missing_first_secondary();
        put(&storage, "store_1", &orig()).await;
        put(&storage, "store_2", &orig()).await;

        let result = storage.copy(&orig(), &dest()).await;
        assert_blames_missing_store(&result);

        for store in ["store_1", "store_2"] {
            assert!(holds(&storage, store, &orig()).await);
            assert!(holds(&storage, store, &dest()).await);
        }
    }

    #[tokio::test]
    async fn upload_stream_attempts_all_secondaries_even_when_first_secondary_fails() {
        let storage = with_missing_first_secondary();
        let stream =
            BytesStream::from_body_stream(futures_util::stream::once(async { Ok(content()) }));

        let result = storage.upload_stream(&orig(), stream).await;
        assert_blames_missing_store(&result);

        assert!(holds(&storage, "store_1", &orig()).await);
        assert!(holds(&storage, "store_2", &orig()).await);
    }

    // ---------------------------------------------------------------
    // reads: mirror falls back to secondaries, backup does not
    // ---------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn download_reads_the_primary(#[values(true, false)] mirror: bool) {
        let storage = storage_for(strategy(mirror, FailIfAny), three([true; 3]));
        storage.upload(&orig(), &content()).await.unwrap();

        let downloaded: String = storage.download(&orig()).await.unwrap();
        assert_eq!(downloaded, "file content");
    }

    #[tokio::test]
    async fn download_falls_back_to_a_secondary_under_mirror() {
        let storage = storage_for(strategy(true, FailIfAny), three([true; 3]));
        storage.upload(&orig(), &content()).await.unwrap();
        for store in ["store_1", "store_2"] {
            storage
                .as_store(store)
                .unwrap()
                .delete(&orig())
                .await
                .unwrap();
        }

        let downloaded: String = storage.download(&orig()).await.unwrap();
        assert_eq!(downloaded, "file content");
    }

    /// A secondary that is only named, never registered, does not stop the
    /// primary from serving reads — but the write that named it still fails.
    #[tokio::test]
    async fn download_from_primary_when_secondaries_are_unregistered() {
        let storage = storage_for(strategy(false, FailIfAny), named(&["store_1"]));

        assert!(storage.upload(&orig(), &content()).await.is_err());

        let downloaded: String = storage.download(&orig()).await.unwrap();
        assert_eq!(downloaded, "file content");

        storage
            .as_store("store_1")
            .unwrap()
            .delete(&orig())
            .await
            .unwrap();
        assert!(storage.download::<String>(&orig()).await.is_err());
    }

    #[tokio::test]
    async fn list_mirror_falls_back_on_empty_or_missing_primary() {
        let path = PathBuf::from("a").join("1.txt");

        let storage = storage_for(
            ReplicatedStrategy::mirror("store_1", Some(vec!["store_2".to_string()]), FailIfAny),
            named(&["store_1", "store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(paths_of(&storage.list(Path::new("a"), true).await.unwrap())
            .contains(&"a/1.txt".into()));

        let storage = storage_for(
            ReplicatedStrategy::mirror(
                "missing_primary",
                Some(vec!["store_2".to_string()]),
                FailIfAny,
            ),
            named(&["store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(paths_of(&storage.list(Path::new("a"), true).await.unwrap())
            .contains(&"a/1.txt".into()));

        // No secondary hit → keep primary `Err` (do not collapse to `Ok([])`).
        let storage = storage_for(
            ReplicatedStrategy::mirror(
                "missing_primary",
                Some(vec!["store_2".to_string()]),
                FailIfAny,
            ),
            named(&["store_2"]),
        );
        assert!(storage.list(Path::new("a"), true).await.is_err());
    }

    #[tokio::test]
    async fn list_backup_stays_on_primary() {
        let path = PathBuf::from("a").join("1.txt");

        let storage = storage_for(
            ReplicatedStrategy::backup("store_1", Some(vec!["store_2".to_string()]), FailIfAny),
            named(&["store_1", "store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(storage.list(Path::new("a"), true).await.unwrap().is_empty());

        let storage = storage_for(
            ReplicatedStrategy::backup(
                "missing_primary",
                Some(vec!["store_2".to_string()]),
                FailIfAny,
            ),
            named(&["store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(storage.list(Path::new("a"), true).await.is_err());
    }

    #[tokio::test]
    async fn list_mirror_skips_empty_secondary_and_prefers_primary_data() {
        let primary_only = PathBuf::from("a").join("primary.txt");
        let secondary_only = PathBuf::from("a").join("secondary.txt");

        // Non-empty primary must win even when a secondary has other keys.
        let storage = storage_for(
            ReplicatedStrategy::mirror("store_1", Some(vec!["store_2".to_string()]), FailIfAny),
            named(&["store_1", "store_2"]),
        );
        put(&storage, "store_1", primary_only.as_path()).await;
        put(&storage, "store_2", secondary_only.as_path()).await;
        let paths = paths_of(&storage.list(Path::new("a"), true).await.unwrap());
        assert!(paths.contains(&"a/primary.txt".into()));
        assert!(!paths.contains(&"a/secondary.txt".into()));

        // Empty secondary must not hide a later secondary that has data.
        let storage = storage_for(
            strategy(true, FailIfAny),
            named(&["store_1", "store_2", "store_3"]),
        );
        put(&storage, "store_3", secondary_only.as_path()).await;
        let paths = paths_of(&storage.list(Path::new("a"), true).await.unwrap());
        assert!(paths.contains(&"a/secondary.txt".into()));
    }

    #[tokio::test]
    async fn stat_mirror_falls_back_backup_does_not() {
        let path = PathBuf::from("users").join("data").join("1.txt");

        let storage = storage_for(
            ReplicatedStrategy::mirror("store_1", Some(vec!["store_2".to_string()]), FailIfAny),
            named(&["store_1", "store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert_eq!(
            storage.stat(path.as_path()).await.unwrap().content_length,
            Some(12)
        );

        let storage = storage_for(
            ReplicatedStrategy::backup("store_1", Some(vec!["store_2".to_string()]), FailIfAny),
            named(&["store_1", "store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(storage.stat(path.as_path()).await.is_err());

        let storage = storage_for(
            ReplicatedStrategy::mirror(
                "missing_primary",
                Some(vec!["store_2".to_string()]),
                FailIfAny,
            ),
            named(&["store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(storage.stat(path.as_path()).await.is_ok());
    }

    #[tokio::test]
    async fn exists_mirror_falls_back_backup_does_not() {
        let path = PathBuf::from("users").join("data").join("1.txt");

        let storage = storage_for(
            ReplicatedStrategy::mirror("store_1", Some(vec!["store_2".to_string()]), FailIfAny),
            named(&["store_1", "store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(storage.exists(path.as_path()).await.unwrap());

        let storage = storage_for(
            ReplicatedStrategy::backup("store_1", Some(vec!["store_2".to_string()]), FailIfAny),
            named(&["store_1", "store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(!storage.exists(path.as_path()).await.unwrap());

        let storage = storage_for(
            ReplicatedStrategy::mirror(
                "missing_primary",
                Some(vec!["store_2".to_string()]),
                FailIfAny,
            ),
            named(&["store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(storage.exists(path.as_path()).await.unwrap());

        let storage = storage_for(
            ReplicatedStrategy::backup(
                "missing_primary",
                Some(vec!["store_2".to_string()]),
                FailIfAny,
            ),
            named(&["store_2"]),
        );
        put(&storage, "store_2", path.as_path()).await;
        assert!(storage.exists(path.as_path()).await.is_err());
    }
}
