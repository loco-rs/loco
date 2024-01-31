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
use object_store::{ Result as ObjectStoreResult};
use bytes::Bytes;
/// A [`Storage`] struct representing the storage options available to the application.
#[derive(Default, Clone)]
pub struct Storage {
    pub local: Option<GenericStorage<LocalFileSystem>>,
    // #[cfg(feature = "store-aws")]
    pub aws_s3: Option<GenericStorage<AmazonS3>>,
    // #[cfg(feature = "store-gcs")]
    pub gcs: Option<GenericStorage<GoogleCloudStorage>>,
    // #[cfg(feature = "store-azure")]
    pub azure: Option<GenericStorage<MicrosoftAzure>>,
}


#[async_trait]
pub trait StoragePut {
    async fn put(&self, file: Bytes, custom_path: Option<&str>) -> ObjectStoreResult<PutResult>;
}

pub struct GenericStorage<T: ObjectStore + Send + Sync + 'static>(Arc<T>, object_store::path::Path);

impl<T> GenericStorage<T>
where
    T: ObjectStore + Send + Sync + 'static,
{
    pub fn new(storage: T, path: object_store::path::Path) -> Self {
        GenericStorage(Arc::new(storage), path)
    }
}
impl<T> Clone for GenericStorage<T>
where
    T: ObjectStore + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        GenericStorage(Arc::clone(&self.0), self.1.clone())
    }
}

#[async_trait]
impl<T> StoragePut for GenericStorage<T>
where
    T: ObjectStore + Send + Sync + 'static,
{
    async fn put(&self, file: Bytes, custom_path: Option<&str>) -> ObjectStoreResult<PutResult> {
        let path = custom_path.map(object_store::path::Path::from).unwrap_or_else(|| self.1.clone());
        self.0.put(&path, file).await
    }
}


impl Storage {
    /// Creates a new [`Storage`] instance with the provided options.
    pub fn new(config: &config::Storage) -> Result<Self> {
        let mut storage = Self::default();
        if let Some(local) = &config.local {
            let local = GenericStorage::new(LocalFileSystem::new(), object_store::path::Path::from(local.path.clone()));
            storage.local = Some(local);
        }
        if let Some(aws_config) = &config.amazon_s3 {
            let aws = AmazonS3Builder::new().with_bucket_name(&aws_config.bucket_name).with_access_key_id(&aws_config.access_key_id).with_secret_access_key(&aws_config.secret_access_key).build().map_err(
                |e| {
                    eprintln!("Failed to create AWS S3 client: {:?}", e);
                    Error::Any("Failed to create AWS S3 client".to_string().into())
                }
            )?;
            let aws_s3 = GenericStorage::new(aws, object_store::path::Path::from(aws_config.path.clone()));
            storage.aws_s3 = Some(aws_s3);
        }
        if let Some(gcs_config) = &config.google_cloud_storage {
            let gcs = GoogleCloudStorageBuilder::new().with_bucket_name(&gcs_config.bucket_name).with_service_account_key(&gcs_config.service_account_key).with_service_account_path(&gcs_config.service_account).build().map_err(
                |e| {
                    eprintln!("Failed to create Google Cloud Storage client: {:?}", e);
                    Error::Any("Failed to create Google Cloud Storage client".to_string().into())
                }
            )?;
            let gcs = GenericStorage::new(gcs, object_store::path::Path::from(gcs_config.path.clone()));
            storage.gcs = Some(gcs);
        }
        if let Some(azure_config) = &config.azure_blob_storage {
            let azure =  MicrosoftAzureBuilder::new().with_container_name(&azure_config.container_name).with_account(&azure_config.account_name).with_access_key(&azure_config.account_key).build().map_err(
                |e| {
                    eprintln!("Failed to create Azure Blob Storage client: {:?}", e);
                    Error::Any("Failed to create Azure Blob Storage client".to_string().into())
                }
            )?; // .with_blob_name(&azure_config.blob_name
            let azure = GenericStorage::new(azure, object_store::path::Path::from(azure_config.path.clone()));
            storage.azure = Some(azure);
        }
        Ok(storage)
    }
}

#[derive(Debug)]
pub enum StorageError {
    AwsClientCreationError(String),
    GcsClientCreationError(String),
    AzureClientCreationError(String),
    LocalStorageError(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            StorageError::AwsClientCreationError(ref err) => write!(f, "AWS S3 Client Creation Error: {}", err),
            StorageError::GcsClientCreationError(ref err) => write!(f, "Google Cloud Storage Client Creation Error: {}", err),
            StorageError::AzureClientCreationError(ref err) => write!(f, "Azure Blob Storage Client Creation Error: {}", err),
            StorageError::LocalStorageError(ref err) => write!(f, "Local Storage Error: {}", err),
            // Handle other errors similarly
        }
    }
}

impl std::error::Error for StorageError {}