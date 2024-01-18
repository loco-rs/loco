use axum::{routing::get, Extension, Router};
use fluent_templates::{ArcLoader, FluentLoader};
use std::sync::Arc;

// Some shared state used throughout our application
pub struct TeraLayer {
    pub tera: tera::Tera,
    pub default_context: tera::Context,
}

pub async fn load_locales() -> Arc<TeraLayer> {
    let arc = ArcLoader::builder("locales", unic_langid::langid!("en-US"))
        .shared_resources(Some(&["./locales/core.ftl".into()]))
        .customize(|bundle| bundle.set_use_isolating(false))
        .build()
        .unwrap();

    let mut tera = tera::Tera::default();
    let ctx = tera::Context::default();
    tera.register_function("fluent", FluentLoader::new(arc));

    Arc::new(TeraLayer {
        tera,
        default_context: ctx,
    })
}

async fn after_routes(router: axum::Router) -> Result<axum::Router, ()> {
    println!("loading locales");
    let st = load_locales().await;
    println!("locales ready");
    Ok(router.layer(Extension(st)))
}
