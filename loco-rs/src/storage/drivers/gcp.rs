use object_store::gcp::GoogleCloudStorageBuilder;

use super::{object_store_adapter::ObjectStoreAdapter, StoreDriver};
use crate::Result;

/// Create new GCP storage.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::gcp;
/// let gcp_driver = gcp::new("key", "account_key", "service_account");
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(
    bucket_name: &str,
    service_account_key: &str,
    service_account: &str,
) -> Result<Box<dyn StoreDriver>> {
    let gcs = GoogleCloudStorageBuilder::new()
        .with_bucket_name(bucket_name)
        .with_service_account_key(service_account_key)
        .with_service_account_path(service_account)
        .build()
        .map_err(Box::from)?;

    Ok(Box::new(ObjectStoreAdapter::new(Box::new(gcs))))
}
