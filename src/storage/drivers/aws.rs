#[cfg(test)]
use core::time::Duration;
use std::sync::Arc;

use object_store::{
    aws::{AmazonS3Builder, AwsCredential},
    StaticCredentialProvider,
};
#[cfg(test)]
use object_store::{BackoffConfig, RetryConfig};

use super::{object_store_adapter::ObjectStoreAdapter, StoreDriver};
use crate::Result;

/// A set of AWS security credentials
pub struct Credential {
    /// AWS_ACCESS_KEY_ID
    pub key_id: String,
    /// AWS_SECRET_ACCESS_KEY
    pub secret_key: String,
    /// AWS_SESSION_TOKEN
    pub token: Option<String>,
}

/// Create new AWS s3 storage with bucket and region.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::aws;
/// let aws_driver = aws::new("bucket_name", "region");
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(bucket_name: &str, region: &str) -> Result<Box<dyn StoreDriver>> {
    let s3 = AmazonS3Builder::new()
        .with_bucket_name(bucket_name)
        .with_region(region)
        .build()
        .map_err(Box::from)?;

    Ok(Box::new(ObjectStoreAdapter::new(Box::new(s3))))
}

/// Create new AWS s3 storage with bucket, region and credentials.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::aws;
/// let credential = aws::Credential {
///    key_id: "".to_string(),
///    secret_key: "".to_string(),
///    token: None
/// };
/// let aws_driver = aws::with_credentials("bucket_name", "region", credential);
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn with_credentials(
    bucket_name: &str,
    region: &str,
    credentials: Credential,
) -> Result<Box<dyn StoreDriver>> {
    let s3 = AmazonS3Builder::new()
        .with_bucket_name(bucket_name)
        .with_region(region)
        .with_credentials(Arc::new(StaticCredentialProvider::new(AwsCredential {
            key_id: credentials.key_id.to_string(),
            secret_key: credentials.secret_key.to_string(),
            token: credentials.token,
        })))
        .build()
        .map_err(Box::from)?;
    Ok(Box::new(ObjectStoreAdapter::new(Box::new(s3))))
}

/// Build store with failure
///
/// # Panics
///
/// Panics if cannot build store
#[cfg(test)]
#[must_use]
pub fn with_failure() -> Box<dyn StoreDriver> {
    let s3 = AmazonS3Builder::new()
        .with_bucket_name("loco-test")
        .with_retry(RetryConfig {
            backoff: BackoffConfig::default(),
            max_retries: 0,
            retry_timeout: Duration::from_secs(0),
        })
        .build()
        .unwrap();

    Box::new(ObjectStoreAdapter::new(Box::new(s3)))
}
