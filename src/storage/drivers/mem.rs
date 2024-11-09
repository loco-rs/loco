use opendal::{services::Memory, Operator};

use super::StoreDriver;
use crate::storage::drivers::opendal_adapter::OpendalAdapter;

/// Create new in-memory storage.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::mem;
/// let mem_storage = mem::new();
/// ```
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    Box::new(OpendalAdapter::new(
        Operator::new(Memory::default())
            .expect("memory service must build with success")
            .finish(),
    ))
}
