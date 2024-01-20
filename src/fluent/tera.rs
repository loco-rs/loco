use crate::config::Config;
use axum::{routing::get, Extension, Router};
use fluent_templates::{ArcLoader, FluentLoader};
use std::sync::Arc;

// Some shared state used throughout our application
pub struct TeraLayer {
    pub tera: tera::Tera,
    pub default_context: tera::Context,
}

pub async fn load_locales(cfg: Config) -> TeraLayer {
    let tera_dir = cfg.tera.clone().unwrap().template_dir.unwrap().dir;
    let mut tera = tera::Tera::new(&tera_dir).unwrap();
    let ctx = tera::Context::default();

    let locales_dir = cfg.tera.clone().unwrap().fluent.unwrap().locales_dir;
    let shared_resources = cfg
        .tera
        .clone()
        .unwrap()
        .fluent
        .unwrap()
        .shared_resources
        .unwrap();
    let arc = ArcLoader::builder(&locales_dir, unic_langid::langid!("en-US"))
        .shared_resources(Some(&[shared_resources.into()]))
        .customize(|bundle| bundle.set_use_isolating(false))
        .build()
        .unwrap();
    tera.register_function("fluent", FluentLoader::new(arc));

    TeraLayer {
        tera,
        default_context: ctx,
    }
}

async fn after_routes(cfg: Config, router: axum::Router) -> Result<axum::Router, ()> {
    println!("loading locales");
    let st = load_locales(cfg).await;
    println!("locales ready");
    Ok(router.layer(Extension(Arc::new(st))))
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::environment::Environment;
    use std::path::PathBuf;

    #[tokio::test]
    async fn can_render_template() {
        let cfg = Config::from_folder(
            &Environment::Test,
            &PathBuf::from("./src/fluent/test_config"),
        )
        .expect("configuration loading");
        println!("{:?}", cfg);

        let mut locales = load_locales(cfg.clone()).await;
        let string = locales
            .tera
            .render_str("hello world", &locales.default_context)
            .unwrap();
        assert_eq!(string, "hello world");

        let mut context = tera::Context::new();
        context.insert("message", &"hello");
        let template = locales.tera.render("test.html", &context).unwrap();

        assert_eq!("<p>hello</p>", template.trim());
    }
}
