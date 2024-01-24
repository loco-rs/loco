use axum::{
    async_trait, extract::FromRequestParts, http::request::Parts, Extension, Router as AxumRouter,
};
use fluent_templates::{ArcLoader, FluentLoader};
use loco_rs::{
    app::{AppContext, Initializer},
    prelude::*,
};

#[derive(Clone, Debug)]
pub struct TeraView {
    pub tera: tera::Tera,
    pub default_context: tera::Context,
}

#[async_trait]
impl<S> FromRequestParts<S> for TeraView
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let Extension(tl): Extension<TeraView> = Extension::from_request_parts(parts, state)
            .await
            .expect("TeraLayer missing. Is the TeraLayer installed?");
        /*
        let locale = parts
            .headers
            .get("Accept-Language")
            .unwrap()
            .to_str()
            .unwrap();
        // BUG: this does not mutate or set anything because of clone
        tl.default_context.clone().insert("locale", &locale);
        */

        Ok(tl)
    }
}

const VIEWS_DIR: &str = "assets/views/**/*.html";
const I18N_DIR: &str = "assets/i18n";
const I18N_SHARED: &str = "assets/i18n/shared.ftl";

pub async fn load_locales() -> Result<TeraView> {
    let mut tera = tera::Tera::new(VIEWS_DIR).unwrap();
    let ctx = tera::Context::default();

    if std::path::Path::new(I18N_DIR).exists() {
        let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
            .shared_resources(Some(&[I18N_SHARED.into()]))
            .customize(|bundle| bundle.set_use_isolating(false))
            .build()
            .unwrap();
        tera.register_function("t", FluentLoader::new(arc));
    }

    Ok(TeraView {
        tera,
        default_context: ctx,
    })
}

async fn impl_after_routes(router: axum::Router) -> Result<axum::Router> {
    println!("loading locales");
    let tera_view = load_locales().await.unwrap();
    println!("locales ready");
    Ok(router.layer(Extension(tera_view)))
}

pub struct ViewTemplatesInitializer;
#[async_trait]
impl Initializer for ViewTemplatesInitializer {
    fn name(&self) -> String {
        "view-templates".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(impl_after_routes(router).await?)
    }
}
