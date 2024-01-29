use std::sync::Arc;

use axum::{
    async_trait, extract::FromRequestParts, http::request::Parts, Extension, Router as AxumRouter,
};
use fluent_templates::{ArcLoader, FluentLoader};
use loco_rs::{
    app::{AppContext, Initializer},
    controller::views::TemplateEngine,
    prelude::*,
};
use serde::Serialize;

// if we ever want to support render.template(engine, key, data), `engine` needs
// to be a trait
impl TemplateEngine for Engine<TeraView> {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        Ok(self
            .inner
            .tera
            .render(key, &tera::Context::from_serialize(data)?)?)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Engine<E> {
    pub inner: Arc<E>,
}
impl<E> Engine<E> {
    /// Creates a new [`Engine`] that wraps the given engine
    pub fn new(engine: E) -> Self {
        let inner = Arc::new(engine);
        Self { inner }
    }
}

impl<E> Clone for Engine<E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<E> From<E> for Engine<E> {
    fn from(inner: E) -> Self {
        Self::new(inner)
    }
}
#[derive(Clone, Debug)]
pub struct TeraView {
    pub tera: tera::Tera,
    pub default_context: tera::Context,
}

#[async_trait]
impl<S, E> FromRequestParts<S> for Engine<E>
where
    S: Send + Sync,
    E: Send + Sync + 'static,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let Extension(tl): Extension<Engine<E>> = Extension::from_request_parts(parts, state)
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
    let eng = Engine::from(tera_view);
    println!("locales ready");
    Ok(router.layer(Extension(eng)))
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
