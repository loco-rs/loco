pub mod backup;
pub mod mirror;
pub mod single;

use std::path::Path;

use bytes::Bytes;

use crate::storage::{stream::BytesStream, Storage, StorageResult};

#[async_trait::async_trait]
pub trait StorageStrategy: Sync + Send {
    async fn upload(&self, storage: &Storage, path: &Path, content: &Bytes) -> StorageResult<()>;
    async fn download(&self, storage: &Storage, path: &Path) -> StorageResult<Bytes>;
    async fn delete(&self, storage: &Storage, path: &Path) -> StorageResult<()>;
    async fn rename(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()>;
    async fn copy(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()>;

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
