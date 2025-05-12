use super::tera_builtins;
use crate::{controller::views::ViewRenderer, Result};
use serde::Serialize;
use std::collections::BTreeMap;

pub static DEFAULT_ASSET_FOLDER: &str = "assets";

// Include the generated templates at the module level
include!(concat!(
    env!("OUT_DIR"),
    "/generated_code/view_templates.rs"
));

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
        Self::from_embedded_templates()
    }

    /// Load and initialize templates from embedded assets
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Adding templates to Tera fails
    /// - There are syntax errors in any template
    pub fn from_embedded_templates() -> Result<Self> {
        let mut tera = tera::Tera::default();

        // Initialize templates in a separate function to reduce complexity
        Self::load_templates_into_tera(&mut tera)?;

        tera_builtins::filters::register_filters(&mut tera);
        let ctx = tera::Context::default();

        Ok(Self {
            tera,
            default_context: ctx,
        })
    }

    /// Helper function to load all embedded templates into Tera engine
    ///
    /// # Errors
    ///
    /// Returns an error if adding a template fails
    fn load_templates_into_tera(tera: &mut tera::Tera) -> Result<()> {
        let templates_map = get_embedded_templates();
        let templates: BTreeMap<_, _> = templates_map.into_iter().collect();
        Self::log_template_info(&templates);
        Self::add_templates_to_tera(tera, templates)
    }

    /// Log information about the templates
    fn log_template_info(templates: &BTreeMap<String, &'static str>) {
        tracing::info!("Initializing embedded templates feature");
        tracing::info!("Found {} embedded templates", templates.len());
    }

    /// Add each template to the Tera engine
    ///
    /// # Errors
    ///
    /// Returns an error if adding any template fails
    fn add_templates_to_tera(
        tera: &mut tera::Tera,
        templates: BTreeMap<String, &'static str>,
    ) -> Result<()> {
        // Add all templates to Tera
        for (name, content) in templates {
            tracing::debug!("Adding template '{}' to Tera", name);
            if let Err(e) = tera.add_raw_template(&name, content) {
                tracing::error!("Failed to add template '{}': {}", name, e);
                return Err(e.into());
            }
        }

        // Ensure templates are properly configured for inheritance
        if let Err(e) = tera.build_inheritance_chains() {
            tracing::error!("Failed to build template inheritance chains: {}", e);
            return Err(e.into());
        }

        Ok(())
    }
}

impl ViewRenderer for TeraView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        let context = tera::Context::from_serialize(data)?;

        // Try to render the requested template
        match self.tera.render(key, &context) {
            Ok(result) => Ok(result),
            Err(e) => {
                // Log error about missing template
                if e.to_string().contains("not found") {
                    tracing::warn!("Template '{}' not found", key);
                    let template_names: Vec<String> =
                        self.tera.get_template_names().map(String::from).collect();
                    tracing::debug!("Available templates: {:?}", template_names);
                }

                // Return the original error
                Err(e.into())
            }
        }
    }
}
