//! This module defines a [`Storage`] trait that can be implemented by LocalStorage, AWS S3, Google Cloud Storage and Azure Blob Storage.

use std::sync::Arc;
use async_trait::async_trait;
use object_store::aws::AmazonS3;
use object_store::azure::MicrosoftAzure;
use object_store::gcp::GoogleCloudStorage;
use object_store::local::LocalFileSystem;
use object_store::{ObjectStore, PutResult};
use object_store::path::Path;
use crate::{config};
use super::Result;

/// A [`Storage`] struct representing the storage options available to the application.
#[derive(Default, Clone)]
pub struct Storage {
    pub local: Option<LocalStorage>,
    // #[cfg(feature = "store-aws")]
    pub aws_s3: Option<AwsS3Storage>,
    // #[cfg(feature = "store-gcs")]
    pub gcs: Option<GcsStorage>,
    // #[cfg(feature = "store-azure")]
    pub azure: Option<AzureStorage>,
}

#[async_trait]
trait StoragePut {
    fn get_storage_path(&self) -> (Box<Arc<dyn ObjectStore>>, object_store::path::Path);
    async fn put(&self, file: bytes::Bytes, custom_path: Option<&str>) -> object_store::Result<PutResult> {
        let (storage, path) = self.get_storage_path();
        let path = custom_path.map(Path::from).unwrap_or(path);
        storage.put(&path, file).await
    }
}
#[derive(Debug, Clone)]
pub struct LocalStorage(Arc<LocalFileSystem>, object_store::path::Path);

impl StoragePut for LocalStorage {
    fn get_storage_path(&self) -> (Box<Arc<dyn ObjectStore>>, object_store::path::Path) {
        let store: Box<Arc<dyn ObjectStore>>= Box::new(self.0.clone());
        (store, self.1.clone())
    }
}

// #[cfg(feature = "store-aws")]
#[derive(Clone)]
pub struct AwsS3Storage(Arc<AmazonS3>, object_store::path::Path);

// #[cfg(feature = "store-gcs")]
#[derive(Clone)]
pub struct GcsStorage(Arc<GoogleCloudStorage>, object_store::path::Path);

// #[cfg(feature = "store-azure")]
#[derive(Clone)]
pub struct AzureStorage(Arc<MicrosoftAzure>, object_store::path::Path);

impl Storage {
    /// Creates a new [`Storage`] instance with the provided options.
    pub fn new(config: &config::Storage) -> Result<Self> {
        let mut storage = Self::default();
        if let Some(local) = &config.local {
            let local = LocalStorage(Arc::new(LocalFileSystem::new()), object_store::path::Path::from(local.path.clone()));
            storage.local = Some(local);
        }


        Ok(storage)
    }
}