use object_store::{memory::InMemory, ObjectStore};

use super::Store;

/// Create new in-memory storage.
#[must_use]
pub fn new() -> Store {
    Store::new((Box::new(InMemory::new()) as Box<dyn ObjectStore>).into())
}
