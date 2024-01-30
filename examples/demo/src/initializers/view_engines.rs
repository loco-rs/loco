use axum::{async_trait, Extension, Router as AxumRouter};
use fluent_templates::{ArcLoader, FluentLoader};
use loco_rs::{
    app::{AppContext, Initializer},
    controller::views::{ViewEngine, ViewRenderer},
    Error, Result,
};
use serde::Serialize;
use tracing::info;

const VIEWS_DIR: &str = "assets/views/**/*.html";
const I18N_DIR: &str = "assets/i18n";
const I18N_SHARED: &str = "assets/i18n/shared.ftl";

#[derive(Clone, Debug)]
pub struct Tera {
    pub tera: tera::Tera,
    pub default_context: tera::Context,
}

impl Tera {
    /// Create a Tera view engine
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    pub fn build() -> Result<Self> {
        let mut tera = tera::Tera::new(VIEWS_DIR)?;
        let ctx = tera::Context::default();
        info!("templates loaded");

        if std::path::Path::new(I18N_DIR).exists() {
            let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
                .shared_resources(Some(&[I18N_SHARED.into()]))
                .customize(|bundle| bundle.set_use_isolating(false))
                .build()
                .map_err(|e| Error::string(&e.to_string()))?;
            tera.register_function("t", FluentLoader::new(arc));
            info!("locales loaded");
        }

        Ok(Self {
            tera,
            default_context: ctx,
        })
    }
}

impl ViewRenderer for Tera {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        Ok(self
            .tera
            .render(key, &tera::Context::from_serialize(data)?)?)
    }
}

pub struct ViewEnginesInitializer;
#[async_trait]
impl Initializer for ViewEnginesInitializer {
    fn name(&self) -> String {
        "view-engines".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router.layer(Extension(ViewEngine::from(Tera::build()?))))
    }
}
