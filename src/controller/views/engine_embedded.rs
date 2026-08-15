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

    /// Create a Tera view engine with a post-processing function, used to
    /// register custom filters and functions (e.g. an i18n `t()`).
    ///
    /// Mirrors the non-embedded engine's constructor of the same name so app
    /// code — including the generated view-engine initializer — works
    /// identically with and without the `embedded_assets` feature.
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails or if the
    /// post-processing function fails
    pub fn build_with_post_process(
        post_process: impl FnMut(&mut tera::Tera) -> Result<()> + Send + Sync + 'static,
    ) -> Result<Self> {
        Self::assemble(post_process)
    }

    /// Attach the Tera view engine with a post-processing function for subsequent instantiation.
    ///
    /// The post-processing function is also run during the call to this method.
    ///
    /// Note that whenever the embedded templates themselves call the registered
    /// filter or function, [`Self::build_with_post_process`] is the only usable
    /// entry point — `build()` would already have failed while loading them.
    ///
    /// # Errors
    ///
    /// This function will return an error if the post-processing function fails
    pub fn post_process(
        self,
        post_process: impl FnMut(&mut tera::Tera) -> Result<()> + Send + Sync + 'static,
    ) -> Result<Self> {
        // Rebuild rather than mutate: registrations have to precede template
        // loading (see `assemble`), and `self`'s templates are already loaded.
        Self::assemble(post_process)
    }

    /// Load and initialize templates from embedded assets
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Adding templates to Tera fails
    /// - There are syntax errors in any template
    pub fn from_embedded_templates() -> Result<Self> {
        Self::assemble(|_| Ok(()))
    }

    /// Builds the engine in the one order Tera 2 permits: register everything
    /// the templates may reference, *then* load the templates.
    ///
    /// Tera 2 resolves filter and function references when a template is added,
    /// so a template calling `t()`, a custom filter, or `get_env` (a Tera 1
    /// built-in Loco now supplies itself) fails to load unless the name is
    /// already registered.
    fn assemble(
        mut post_process: impl FnMut(&mut tera::Tera) -> Result<()> + Send + Sync + 'static,
    ) -> Result<Self> {
        let mut tera = crate::tera::instance();
        post_process(&mut tera)?;
        Self::load_templates_into_tera(&mut tera)?;

        Ok(Self {
            tera,
            default_context: tera::Context::default(),
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
        // Register the whole set in ONE call. Tera 2 resolves inheritance as
        // templates are added and rejects a child whose parent it has not seen,
        // so adding them one at a time would fail whenever a child sorts before
        // its `{% extends %}` parent.
        for name in templates.keys() {
            tracing::debug!("Adding template '{}' to Tera", name);
        }
        tera.add_raw_templates(templates).map_err(|e| {
            tracing::error!("Failed to add templates: {}", e);
            crate::Error::from(e)
        })?;

        Ok(())
    }
}

impl ViewRenderer for TeraView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        let context = tera::Context::from_serialize(&data)?;

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
