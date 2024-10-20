pub mod backup;
pub mod mirror;
pub mod single;

use std::path::Path;

use bytes::Bytes;

use crate::storage::{Storage, StorageResult};

#[async_trait::async_trait]
pub trait StorageStrategy: Sync + Send {
    async fn upload(&self, storage: &Storage, path: &Path, content: &Bytes) -> StorageResult<()>;
    async fn download(&self, storage: &Storage, path: &Path) -> StorageResult<Bytes>;
    async fn delete(&self, storage: &Storage, path: &Path) -> StorageResult<()>;
    async fn rename(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()>;
    async fn copy(&self, storage: &Storage, from: &Path, to: &Path) -> StorageResult<()>;
}
