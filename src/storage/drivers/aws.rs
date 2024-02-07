#[cfg(test)]
use core::time::Duration;
use std::sync::Arc;

use object_store::{
    aws::{AmazonS3Builder, AwsCredential},
    ObjectStore, StaticCredentialProvider,
};
#[cfg(test)]
use object_store::{BackoffConfig, RetryConfig};

use super::Store;
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
/// # Errors
///
/// When could not initialize the client instance
pub fn new(bucket_name: &str, region: &str) -> Result<Store> {
    let s3 = AmazonS3Builder::new()
        .with_bucket_name(bucket_name)
        .with_region(region)
        .build()
        .map_err(Box::from)?;

    Ok(Store::new((Box::new(s3) as Box<dyn ObjectStore>).into()))
}

/// Create new AWS s3 storage with bucket, region and credentials.
///
/// # Errors
///
/// When could not initialize the client instance
pub fn with_credentials(bucket_name: &str, region: &str, credentials: Credential) -> Result<Store> {
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

    Ok(Store::new((Box::new(s3) as Box<dyn ObjectStore>).into()))
}

#[cfg(test)]
pub fn with_failure() -> Store {
    let s3 = AmazonS3Builder::new()
        .with_bucket_name("loco-test")
        .with_retry(RetryConfig {
            backoff: BackoffConfig::default(),
            max_retries: 0,
            retry_timeout: Duration::from_secs(0),
        })
        .build()
        .unwrap();

    Store::new((Box::new(s3) as Box<dyn ObjectStore>).into())
}
