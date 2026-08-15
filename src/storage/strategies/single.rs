//! # Single Storage Strategy Implementation
//!
//! This module provides an implementation of the [`StorageStrategy`] for a
//! single storage strategy.
use std::path::Path;

use bytes::Bytes;

use crate::storage::{drivers::ListEntry, strategies::StorageStrategy, Storage, StorageResult};

/// Represents a single storage strategy.
#[derive(Clone)]
pub struct SingleStrategy {
    pub primary: String,
}

impl SingleStrategy {
    /// Creates a new instance of `SingleStrategy` with the specified primary
    /// storage identifier.
    #[must_use]
    pub fn new(primary: &str) -> Self {
        Self {
            primary: primary.to_string(),
        }
    }
}

/// Implementation of `StorageStrategy` for a single storage strategy.
#[async_trait::async_trait]
impl StorageStrategy for SingleStrategy {
    /// Uploads content to the primary storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn upload(&self, storage: &Storage, path: &Path, content: &Bytes) -> StorageResult<()> {
        storage
            .as_store_err(&self.primary)?
            .upload(path, content)
            .await?;
        Ok(())
    }

    /// Downloads content
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn download(&self, storage: &Storage, path: &Path) -> StorageResult<Bytes> {
        let store = storage.as_store_err(&self.primary)?;
        Ok(store.get(path).await?.bytes().await?)
    }

    /// Deletes the given path
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn delete(&self, storage: &Storage, path: &Path) -> StorageResult<()> {
        Ok(storage.as_store_err(&self.primary)?.delete(path).await?)
    }

    /// Renames the file name
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn rename(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()> {
        Ok(storage
            .as_store_err(&self.primary)?
            .rename(from, to)
            .await?)
    }

    /// Copy file from the given path to the new path
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn copy(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()> {
        Ok(storage.as_store_err(&self.primary)?.copy(from, to).await?)
    }

    /// Checks if content exists at the given path in the primary storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn exists(&self, storage: &Storage, path: &Path) -> StorageResult<bool> {
        storage.as_store_err(&self.primary)?.exists(path).await
    }

    /// Lists entries under the given prefix in the primary storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn list(
        &self,
        storage: &Storage,
        path: &Path,
        recursive: bool,
    ) -> StorageResult<Vec<ListEntry>> {
        storage
            .as_store_err(&self.primary)?
            .list(path, recursive)
            .await
    }

    /// Retrieves metadata for a single path in the primary storage.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] indicating of the operation status.
    async fn stat(&self, storage: &Storage, path: &Path) -> StorageResult<ListEntry> {
        storage.as_store_err(&self.primary)?.stat(path).await
    }

    /// Downloads content as a stream from the primary storage
    ///
    /// # Errors
    ///
    /// Returns a [`StorageResult`] with the stream
    async fn download_stream(
        &self,
        storage: &Storage,
        path: &Path,
    ) -> StorageResult<super::super::stream::BytesStream> {
        storage.as_store_err(&self.primary)?.get_stream(path).await
    }

    /// Uploads content from a stream to the primary storage
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
        storage
            .as_store_err(&self.primary)?
            .upload_stream(path, stream)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
    };

    use super::*;
    use crate::storage::{drivers, Storage};

    #[tokio::test]
    async fn can_upload() {
        let store = drivers::mem::new();

        let strategy = Box::new(SingleStrategy::new("default")) as Box<dyn StorageStrategy>;

        let storage = Storage::new(BTreeMap::from([("default".to_string(), store)]), strategy);

        let store = storage.as_store("default").unwrap();
        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn can_download() {
        let store = drivers::mem::new();

        let strategy = Box::new(SingleStrategy::new("default")) as Box<dyn StorageStrategy>;

        let storage = Storage::new(BTreeMap::from([("default".to_string(), store)]), strategy);

        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        let store = storage.as_store("default").unwrap();
        assert!(store.upload(path.as_path(), &file_content).await.is_ok());

        let download_file: String = storage.download(path.as_path()).await.unwrap();
        assert_eq!(download_file, file_content);
    }

    #[tokio::test]
    async fn can_delete() {
        let store = drivers::mem::new();

        let strategy = Box::new(SingleStrategy::new("default")) as Box<dyn StorageStrategy>;

        let storage = Storage::new(BTreeMap::from([("default".to_string(), store)]), strategy);

        let store = storage.as_store("default").unwrap();
        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(store.upload(path.as_path(), &file_content).await.is_ok());

        assert!(store.exists(path.as_path()).await.unwrap());

        assert!(storage.delete(path.as_path()).await.is_ok());

        assert!(!store.exists(path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn can_rename_file_path() {
        let store = drivers::mem::new();

        let strategy = Box::new(SingleStrategy::new("default")) as Box<dyn StorageStrategy>;

        let storage = Storage::new(BTreeMap::from([("default".to_string(), store)]), strategy);

        let store = storage.as_store("default").unwrap();
        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store.exists(orig_path.as_path()).await.unwrap());

        let new_path = PathBuf::from("users").join("data-2").join("2.txt");
        assert!(storage
            .rename(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(!store.exists(orig_path.as_path()).await.unwrap());
        assert!(store.exists(new_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn can_copy_file_path() {
        let store = drivers::mem::new();

        let strategy = Box::new(SingleStrategy::new("default")) as Box<dyn StorageStrategy>;

        let storage = Storage::new(BTreeMap::from([("default".to_string(), store)]), strategy);

        let store = storage.as_store("default").unwrap();
        let orig_path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(orig_path.as_path(), &file_content)
            .await
            .is_ok());

        assert!(store.exists(orig_path.as_path()).await.unwrap());

        let new_path = PathBuf::from("users").join("data-2").join("2.txt");
        assert!(storage
            .copy(orig_path.as_path(), new_path.as_path())
            .await
            .is_ok());

        assert!(store.exists(orig_path.as_path()).await.unwrap());
        assert!(store.exists(new_path.as_path()).await.unwrap());
    }

    fn single_storage() -> Storage {
        Storage::new(
            BTreeMap::from([("default".to_string(), drivers::mem::new())]),
            Box::new(SingleStrategy::new("default")) as Box<dyn StorageStrategy>,
        )
    }

    #[tokio::test]
    async fn can_check_exists() {
        let storage = single_storage();
        let path = PathBuf::from("users").join("data").join("1.txt");
        let missing_path = PathBuf::from("users").join("data").join("missing.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());
        assert!(storage.exists(path.as_path()).await.unwrap());
        assert!(!storage.exists(missing_path.as_path()).await.unwrap());
    }

    #[tokio::test]
    async fn can_list_recursive() {
        let storage = single_storage();
        let file_content = Bytes::from("file content");
        let path_1 = PathBuf::from("a").join("1.txt");
        let path_2 = PathBuf::from("a").join("b").join("2.txt");

        assert!(storage
            .upload(path_1.as_path(), &file_content)
            .await
            .is_ok());
        assert!(storage
            .upload(path_2.as_path(), &file_content)
            .await
            .is_ok());

        let paths: Vec<_> = storage
            .list(Path::new("a"), true)
            .await
            .unwrap()
            .iter()
            .map(|e| e.path.as_str().to_string())
            .collect();
        assert!(paths.iter().any(|p| p == "a/1.txt"));
        assert!(paths.iter().any(|p| p == "a/b/2.txt"));
    }

    #[tokio::test]
    async fn can_list_non_recursive() {
        let storage = single_storage();
        let file_content = Bytes::from("file content");
        let path_1 = PathBuf::from("a").join("1.txt");
        let path_2 = PathBuf::from("a").join("b").join("2.txt");

        assert!(storage
            .upload(path_1.as_path(), &file_content)
            .await
            .is_ok());
        assert!(storage
            .upload(path_2.as_path(), &file_content)
            .await
            .is_ok());

        let paths: Vec<_> = storage
            .list(Path::new("a"), false)
            .await
            .unwrap()
            .iter()
            .map(|e| e.path.as_str().to_string())
            .collect();
        assert!(paths.iter().any(|p| p == "a/1.txt"));
        assert!(paths.iter().any(|p| p == "a/b/"));
        assert!(!paths.iter().any(|p| p == "a/b/2.txt"));
    }

    #[tokio::test]
    async fn can_stat() {
        let storage = single_storage();
        let path = PathBuf::from("users").join("data").join("1.txt");
        let file_content = Bytes::from("file content");

        assert!(storage.upload(path.as_path(), &file_content).await.is_ok());
        let entry = storage.stat(path.as_path()).await.unwrap();
        assert_eq!(entry.content_length, Some(file_content.len() as u64));
        assert!(!entry.is_dir);
    }

    #[tokio::test]
    async fn stat_missing_path_errors() {
        let storage = single_storage();
        let missing = PathBuf::from("users").join("data").join("missing.txt");
        assert!(storage.stat(missing.as_path()).await.is_err());
    }

    #[tokio::test]
    async fn list_missing_prefix_returns_empty() {
        let storage = single_storage();
        assert!(storage
            .list(Path::new("missing"), true)
            .await
            .unwrap()
            .is_empty());
    }

    /// `is_dir` is the only thing separating a common prefix from a key, and
    /// the recursive/non-recursive tests above match on `path` alone — which
    /// a driver returning `is_dir: false` for everything would still pass.
    #[tokio::test]
    async fn non_recursive_listing_marks_the_common_prefix_as_a_directory() {
        let storage = single_storage();
        let file_content = Bytes::from("file content");

        assert!(storage
            .upload(Path::new("a/1.txt"), &file_content)
            .await
            .is_ok());
        assert!(storage
            .upload(Path::new("a/b/2.txt"), &file_content)
            .await
            .is_ok());

        let entries = storage.list(Path::new("a"), false).await.unwrap();

        let dir = entries
            .iter()
            .find(|entry| entry.path == "a/b/")
            .expect("the child prefix is listed");
        assert!(dir.is_dir);

        let file = entries
            .iter()
            .find(|entry| entry.path == "a/1.txt")
            .expect("the key is listed");
        assert!(!file.is_dir);
    }

    /// The `*_with_policy` variants are the only way to override the strategy
    /// per call. Nothing else exercises them, so a facade method wired to
    /// `self.strategy` instead of its `strategy` argument would go unnoticed.
    #[tokio::test]
    async fn with_policy_routes_to_the_strategy_it_is_handed() {
        let storage = Storage::new(
            BTreeMap::from([
                ("default".to_string(), drivers::mem::new()),
                ("other".to_string(), drivers::mem::new()),
            ]),
            Box::new(SingleStrategy::new("default")) as Box<dyn StorageStrategy>,
        );
        let other = SingleStrategy::new("other");
        let path = PathBuf::from("only-in-default.txt");

        assert!(storage
            .upload(path.as_path(), &Bytes::from("file content"))
            .await
            .is_ok());

        assert!(storage.exists(path.as_path()).await.unwrap());
        assert!(!storage
            .exists_with_policy(path.as_path(), &other)
            .await
            .unwrap());

        assert_eq!(storage.list(Path::new(""), true).await.unwrap().len(), 1);
        assert!(storage
            .list_with_policy(Path::new(""), true, &other)
            .await
            .unwrap()
            .is_empty());

        assert!(storage.stat(path.as_path()).await.is_ok());
        assert!(storage
            .stat_with_policy(path.as_path(), &other)
            .await
            .is_err());
    }
}
