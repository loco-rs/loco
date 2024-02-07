use object_store::{gcp::GoogleCloudStorageBuilder, ObjectStore};

use super::Store;
use crate::Result;

/// Create new GCP storage.
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(bucket_name: &str, service_account_key: &str, service_account: &str) -> Result<Store> {
    let gcs = GoogleCloudStorageBuilder::new()
        .with_bucket_name(bucket_name)
        .with_service_account_key(service_account_key)
        .with_service_account_path(service_account)
        .build()
        .map_err(Box::from)?;

    Ok(Store::new((Box::new(gcs) as Box<dyn ObjectStore>).into()))
}
