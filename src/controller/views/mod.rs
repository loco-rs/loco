use serde::Serialize;

use crate::Result;

#[cfg(feature = "with-db")]
pub mod pagination;

pub trait TemplateEngine {
    /// Render a template located by `key`
    ///
    /// # Errors
    ///
    /// This function will return an error if render fails
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String>;
}
