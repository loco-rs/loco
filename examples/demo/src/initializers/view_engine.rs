use axum::{async_trait, Extension, Router as AxumRouter};
use fluent_templates::{ArcLoader, FluentLoader};
use loco_rs::{
    app::{AppContext, Initializer},
    controller::views::{engines, ViewEngine},
    Error, Result,
};
use tracing::info;

const I18N_DIR: &str = "assets/i18n";
const I18N_SHARED: &str = "assets/i18n/shared.ftl";

/*
finalizing tasks
[ ] extract this initializer as standard in saas starter, add to app hooks
[ ] add assets folder structure to saas starter
[ ] document this view engine, incl. i18n and folder locations
[ ] document how to build a custom view engine
[?] view generator, needs to be tera specific, what about using this generator in
    starters that have no views?
 */

pub struct ViewEngineInitializer;
#[async_trait]
impl Initializer for ViewEngineInitializer {
    fn name(&self) -> String {
        "view-engines".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let mut tera_engine = engines::TeraView::build()?;
        if std::path::Path::new(I18N_DIR).exists() {
            let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
                .shared_resources(Some(&[I18N_SHARED.into()]))
                .customize(|bundle| bundle.set_use_isolating(false))
                .build()
                .map_err(|e| Error::string(&e.to_string()))?;
            tera_engine
                .tera
                .register_function("t", FluentLoader::new(arc));
            info!("locales loaded");
        }

        Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
    }
}
