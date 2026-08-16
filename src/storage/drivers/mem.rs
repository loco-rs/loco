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
///
/// # Panics
///
/// Panics if the memory service built failed.
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    // opendal 0.58: Operator::new returns a finished Operator (no .finish()).
    Box::new(OpendalAdapter::new(
        Operator::new(Memory::default()).expect("memory service must build with success"),
    ))
}
