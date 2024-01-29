use axum::{
    async_trait, extract::FromRequestParts, http::request::Parts, Extension, Router as AxumRouter,
};
use fluent_templates::{ArcLoader, FluentLoader};
use loco_rs::{
    app::{AppContext, Initializer},
    controller::views::{Engine, TemplateEngine},
    prelude::*,
};
use serde::Serialize;

const VIEWS_DIR: &str = "assets/views/**/*.html";
const I18N_DIR: &str = "assets/i18n";
const I18N_SHARED: &str = "assets/i18n/shared.ftl";

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
        let mut tera = tera::Tera::new(VIEWS_DIR)?;
        let ctx = tera::Context::default();

        if std::path::Path::new(I18N_DIR).exists() {
            println!("loading locales");
            let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
                .shared_resources(Some(&[I18N_SHARED.into()]))
                .customize(|bundle| bundle.set_use_isolating(false))
                .build()
                .map_err(|e| Error::string(&e.to_string()))?;
            println!("locales ready");
            tera.register_function("t", FluentLoader::new(arc));
        }

        Ok(Self {
            tera,
            default_context: ctx,
        })
    }
}

impl TemplateEngine for TeraView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        Ok(self
            .tera
            .render(key, &tera::Context::from_serialize(data)?)?)
    }
}

pub struct ViewTemplatesInitializer;
#[async_trait]
impl Initializer for ViewTemplatesInitializer {
    fn name(&self) -> String {
        "view-templates".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router.layer(Extension(Engine::from(TeraView::build()?))))
    }
}
