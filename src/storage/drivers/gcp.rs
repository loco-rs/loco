use opendal::{services::Gcs, Operator};

use super::StoreDriver;
use crate::storage::{drivers::opendal_adapter::OpendalAdapter, StorageResult};

/// Create new GCP storage.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::gcp;
/// let gcp_driver = gcp::new("key", "credential_path");
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(bucket_name: &str, credential_path: &str) -> StorageResult<Box<dyn StoreDriver>> {
    let gcs = Gcs::default()
        .bucket(bucket_name)
        .credential_path(credential_path);

    // opendal 0.58: Operator::new returns a finished Operator (no .finish()).
    Ok(Box::new(OpendalAdapter::new(Operator::new(gcs)?)))
}

/// Create GCP storage using Application Default Credentials.
///
/// `OpenDAL` discovers credentials from `GOOGLE_APPLICATION_CREDENTIALS`, the
/// local gcloud ADC file, or the Google metadata service. It loads credentials
/// when a storage request needs them, not when this function returns.
///
/// Plain user ADC and default metadata credentials cannot generate presigned
/// URLs through this constructor. Service-account JSON keys support presigning
/// whether discovered through ADC or supplied explicitly to [`new`].
///
/// # Examples
/// ```
/// use loco_rs::storage::drivers::gcp;
/// let driver = gcp::with_default_credentials("bucket_name")?;
/// # Ok::<(), loco_rs::storage::StorageError>(())
/// ```
///
/// # Errors
///
/// Returns an error if the underlying `OpenDAL` operator cannot be built, for
/// example if the bucket name is empty. Authentication and authorization errors
/// can occur later, when a storage operation runs.
pub fn with_default_credentials(bucket_name: &str) -> StorageResult<Box<dyn StoreDriver>> {
    let gcs = Gcs::default().bucket(bucket_name);
    Ok(Box::new(OpendalAdapter::new(Operator::new(gcs)?)))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bytes::Bytes;

    use super::{new, with_default_credentials};

    #[test]
    fn constructors_validate_bucket_without_loading_credentials() {
        let tree = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("temporary directory");
        let missing = tree.root.join("missing-credentials.json");
        let missing = missing.to_str().expect("UTF-8 path");

        assert!(new("test-bucket", missing).is_ok());
        assert!(with_default_credentials("test-bucket").is_ok());
        assert!(new("", missing).is_err());
        assert!(with_default_credentials("").is_err());
    }

    #[tokio::test]
    #[ignore = "requires LOCO_TEST_GCS_BUCKET and ADC with object read/write/delete permissions"]
    async fn default_credentials_roundtrip() {
        let bucket = std::env::var("LOCO_TEST_GCS_BUCKET").expect("set LOCO_TEST_GCS_BUCKET");
        let store = with_default_credentials(&bucket).expect("build GCS driver");
        let object = format!("loco-adc-test/{}.txt", uuid::Uuid::new_v4());
        let path = Path::new(&object);
        let body = Bytes::from_static(b"Loco ADC round-trip");

        store.upload(path, &body).await.expect("upload object");
        let downloaded = async { store.get(path).await?.bytes().await }.await;
        store.delete(path).await.expect("delete test object");
        assert_eq!(downloaded.expect("download object"), body);
    }
}
