use object_store::azure::MicrosoftAzureBuilder;

use super::{object_store_adapter::ObjectStoreAdapter, StoreDriver};
use crate::Result;

/// Create new Azure storage.
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(
    container_name: &str,
    account_name: &str,
    access_key: &str,
) -> Result<Box<dyn StoreDriver>> {
    let azure = MicrosoftAzureBuilder::new()
        .with_container_name(container_name)
        .with_account(account_name)
        .with_access_key(access_key)
        .build()
        .map_err(Box::from)?;

    Ok(Box::new(ObjectStoreAdapter::new(Box::new(azure))))
}
