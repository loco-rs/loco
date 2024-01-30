use serde::Serialize;
use tracing::info;

use crate::{controller::views::ViewRenderer, Result};

const VIEWS_DIR: &str = "assets/views/**/*.html";

#[derive(Clone, Debug)]
pub struct TeraView {
    pub tera: tera::Tera,
    pub default_context: tera::Context,
}

impl TeraView {
    /// Create a Tera view engine
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    pub fn build() -> Result<Self> {
        let tera = tera::Tera::new(VIEWS_DIR)?;
        let ctx = tera::Context::default();
        Ok(Self {
            tera,
            default_context: ctx,
        })
    }
}

impl ViewRenderer for TeraView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        Ok(self
            .tera
            .render(key, &tera::Context::from_serialize(data)?)?)
    }
}
