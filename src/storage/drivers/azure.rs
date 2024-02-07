use object_store::{azure::MicrosoftAzureBuilder, ObjectStore};

use super::Store;
use crate::Result;

/// Create new Azure storage.
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(container_name: &str, account_name: &str, access_key: &str) -> Result<Store> {
    let azure = MicrosoftAzureBuilder::new()
        .with_container_name(container_name)
        .with_account(account_name)
        .with_access_key(access_key)
        .build()
        .map_err(Box::from)?;

    Ok(Store::new((Box::new(azure) as Box<dyn ObjectStore>).into()))
}
