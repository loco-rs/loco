use opendal::{services::Azblob, Operator};

use super::StoreDriver;
use crate::storage::{drivers::opendal_adapter::OpendalAdapter, StorageResult};

/// Create new Azure storage.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::azure;
/// let azure_driver = azure::new("name", "account_name", "access_key", "endpoint");
/// ```
///
/// # Errors
///
/// When could not initialize the client instance
pub fn new(
    container_name: &str,
    account_name: &str,
    access_key: &str,
    endpoint: &str,
) -> StorageResult<Box<dyn StoreDriver>> {
    let azure = Azblob::default()
        .container(container_name)
        .account_name(account_name)
        .account_key(access_key)
        .endpoint(endpoint);

    Ok(Box::new(OpendalAdapter::new(
        Operator::new(azure)?.finish(),
    )))
}
