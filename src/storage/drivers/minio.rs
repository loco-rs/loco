use opendal::{services::S3 as S3Minio, Operator};

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

/// Create new minio storage with bucket and url.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::minio;
/// let minio_driver = minio::new("bucket_name", "region");
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(bucket_name: &str, endpoint: &str) -> StorageResult<Box<dyn StoreDriver>> {
    let minio = S3Minio::default().bucket(bucket_name).endpoint(endpoint);

    Ok(Box::new(OpendalAdapter::new(Operator::new(minio)?.finish())))
}

/// Create new Minio storage with bucket, region and credentials.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::minio;
/// let credential = minio::Credential {
///    key_id: "".to_string(),
///    secret_key: "".to_string(),
///    endpoint: "http://localhost:9000".to_string()
/// };
/// let minio_driver = minio::with_credentials("bucket_name",credential);
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn with_bucket_and_credentials(
    bucket_name: &str,
    credentials: Credential,
) -> StorageResult<Box<dyn StoreDriver>> {
    let minio = S3Minio::default()
        .bucket(bucket_name)
        .endpoint(&credentials.endpoint)
        .region("auto")
        .access_key_id(&credentials.key_id)
        .secret_access_key(&credentials.secret_key);
    Ok(Box::new(OpendalAdapter::new(Operator::new(minio)?.finish())))
}

/// Build store with failure
///
/// # Panics
///
/// Panics if cannot build store
#[cfg(test)]
#[must_use]
pub fn with_failure() -> Box<dyn StoreDriver> {
    let minio = S3Minio::default()
        .bucket("loco-test")
        .region("ap-south-1")
        .allow_anonymous()
        .disable_ec2_metadata();

    Box::new(OpendalAdapter::new(Operator::new(minio).unwrap().finish()))
}
