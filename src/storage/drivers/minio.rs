use opendal::{services::S3, Operator};

use super::{opendal_adapter::OpendalAdapter, StoreDriver};
use crate::storage::StorageResult;

/// A set of Minio security credentials
#[derive(Debug)]
pub struct Credential {
    /// `Minio_ACCESS_KEY_ID`
    pub key_id: String,
    /// `Minio_SECRET_ACCESS_KEY`
    pub secret_key: String,
    /// `Minio_SESSION_TOKEN`
    pub endpoint: String
}

/// Create new Minio s3 storage with bucket and region.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::Minio;
/// let Minio_driver = Minio::new("bucket_name", "region");
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(bucket_name: &str, endpoint: &str) -> StorageResult<Box<dyn StoreDriver>> {
    let s3 = S3::default().bucket(bucket_name).endpoint(endpoint);

    Ok(Box::new(OpendalAdapter::new(Operator::new(s3)?.finish())))
}

/// Create new Minio s3 storage with bucket, region and credentials.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::Minio;
/// let credential = Minio::Credential {
///    key_id: "".to_string(),
///    secret_key: "".to_string(),
///    token: None
/// };
/// let Minio_driver = Minio::with_credentials("bucket_name", "region", credential);
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn with_credentials(
    bucket_name: &str,
    endpoint: &str,
    credentials: Credential,
) -> StorageResult<Box<dyn StoreDriver>> {
    let mut s3 = S3::default()
        .bucket(bucket_name)
        .endpoint(endpoint)
        .access_key_id(&credentials.key_id)
        .secret_access_key(&credentials.secret_key);
    if let Some(token) = credentials.token {
        s3 = s3.session_token(&token);
    }
    Ok(Box::new(OpendalAdapter::new(Operator::new(s3)?.finish())))
}

/// Build store with failure
///
/// # Panics
///
/// Panics if cannot build store
#[cfg(test)]
#[must_use]
pub fn with_failure() -> Box<dyn StoreDriver> {
    let s3 = S3::default()
        .bucket("loco-test")
        .region("ap-south-1")
        .allow_anonymous()
        .disable_ec2_metadata();

    Box::new(OpendalAdapter::new(Operator::new(s3).unwrap().finish()))
}
