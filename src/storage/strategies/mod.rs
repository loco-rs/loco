pub mod replicated;
pub mod single;

use std::{path::Path, time::Duration};

use bytes::Bytes;

use crate::storage::{
    drivers::{ListEntry, PresignPutOptions, PresignedRequest},
    stream::BytesStream,
    Storage, StorageError, StorageResult,
};

#[async_trait::async_trait]
pub trait StorageStrategy: Sync + Send {
    async fn upload(&self, storage: &Storage, path: &Path, content: &Bytes) -> StorageResult<()>;
    async fn download(&self, storage: &Storage, path: &Path) -> StorageResult<Bytes>;
    async fn delete(&self, storage: &Storage, path: &Path) -> StorageResult<()>;
    async fn rename(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()>;
    async fn copy(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()>;
    async fn exists(&self, storage: &Storage, path: &Path) -> StorageResult<bool>;
    async fn list(
        &self,
        storage: &Storage,
        path: &Path,
        recursive: bool,
    ) -> StorageResult<Vec<ListEntry>>;
    async fn stat(&self, storage: &Storage, path: &Path) -> StorageResult<ListEntry>;
    async fn presign_get(
        &self,
        storage: &Storage,
        path: &Path,
        expire: Duration,
    ) -> StorageResult<PresignedRequest> {
        let _ = (storage, path, expire);
        Err(StorageError::Any(
            "presign_get is not supported by this storage driver".into(),
        ))
    }
    async fn presign_put(
        &self,
        storage: &Storage,
        path: &Path,
        expire: Duration,
        options: PresignPutOptions,
    ) -> StorageResult<PresignedRequest> {
        let _ = (storage, path, expire, options);
        Err(StorageError::Any(
            "presign_put is not supported by this storage driver".into(),
        ))
    }

    /// Download content as a stream for memory-efficient large file handling.
    ///
    /// Strategies must implement this method to support streaming downloads.
    async fn download_stream(&self, storage: &Storage, path: &Path) -> StorageResult<BytesStream>;

    /// Upload content from a stream for memory-efficient large file handling.
    ///
    /// Strategies must implement this method to support streaming uploads.
    async fn upload_stream(
        &self,
        storage: &Storage,
        path: &Path,
        stream: BytesStream,
    ) -> StorageResult<()>;
}
