use object_store::memory::InMemory;

use super::{object_store_adapter::ObjectStoreAdapter, StoreDriver};

/// Create new in-memory storage.
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    Box::new(ObjectStoreAdapter::new(Box::new(InMemory::new())))
}
