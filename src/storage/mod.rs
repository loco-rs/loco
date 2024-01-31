//! This module defines a [`Storage`] trait that can be implemented by LocalStorage, AWS S3, Google Cloud Storage and Azure Blob Storage.

use std::sync::Arc;
use async_trait::async_trait;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::azure::{MicrosoftAzure, MicrosoftAzureBuilder};
use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};
use object_store::local::LocalFileSystem;
use object_store::{ObjectStore, PutResult};
use object_store::path::Path;
use crate::{config, Error};
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
        let store: Box<Arc<dyn ObjectStore>> = Box::new(self.0.clone());
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
        if let Some(aws_config) = &config.amazon_s3 {
            let aws = AmazonS3Builder::new().with_bucket_name(&aws_config.bucket_name).with_access_key_id(&aws_config.access_key_id).with_secret_access_key(&aws_config.secret_access_key).build().map_err(
                |e| {
                    eprintln!("Failed to create AWS S3 client: {:?}", e);
                    Error::Any("Failed to create AWS S3 client".to_string().into())
                }
            )?;
            let aws_s3 = AwsS3Storage(Arc::new(aws), object_store::path::Path::from(aws_config.path.clone()));
            storage.aws_s3 = Some(aws_s3);
        }
        if let Some(gcs_config) = &config.google_cloud_storage {
            let gcs = GoogleCloudStorageBuilder::new().with_bucket_name(&gcs_config.bucket_name).with_service_account_key(&gcs_config.service_account_key).with_service_account_path(&gcs_config.service_account).build().map_err(
                |e| {
                    eprintln!("Failed to create Google Cloud Storage client: {:?}", e);
                    Error::Any("Failed to create Google Cloud Storage client".to_string().into())
                }
            )?;
            let gcs = GcsStorage(Arc::new(gcs), object_store::path::Path::from(gcs_config.path.clone()));
            storage.gcs = Some(gcs);
        }
        if let Some(azure_config) = &config.azure_blob_storage {
            let azure =  MicrosoftAzureBuilder::new().with_container_name(&azure_config.container_name).with_account(&azure_config.account_name).with_access_key(&azure_config.account_key).build().map_err(
                |e| {
                    eprintln!("Failed to create Azure Blob Storage client: {:?}", e);
                    Error::Any("Failed to create Azure Blob Storage client".to_string().into())
                }
            )?; // .with_blob_name(&azure_config.blob_name
            let azure = AzureStorage(Arc::new(azure), object_store::path::Path::from(azure_config.path.clone()));
            storage.azure = Some(azure);
        }
        Ok(storage)
    }
}