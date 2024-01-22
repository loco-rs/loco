use crate::{config::Config, errors::Error};
use axum::{async_trait, extract::FromRequestParts, http::request::Parts, Extension};
use fluent_templates::{ArcLoader, FluentLoader};
use std::sync::Arc;

// Tera Layer, which includes the `tera` field with
// - directory of templates
// - `fluent` function added
// - fluent directory of locales
// and the `default_context` which can be extended with local info per request
#[derive(Clone, Debug)]
pub struct TeraLayer {
    pub tera: tera::Tera,
    pub default_context: tera::Context,
}

// Extractor for TeraLayer which checks the request for the `Accept-Language` header
// and sets that in the context
#[async_trait]
impl<S> FromRequestParts<S> for TeraLayer
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(tl): Extension<TeraLayer> = Extension::from_request_parts(parts, state)
            .await
            .expect("TeraLayer missing. Is the TeraLayer installed?");
        let locale = parts
            .headers
            .get("Accept-Language")
            .unwrap()
            .to_str()
            .unwrap();
        tl.default_context.clone().insert("locale", &locale);

        Ok(tl)
    }
}

// Function that loads the templates and locales directory into the TeraLayer
// to be added to the router as a layer
pub async fn load_locales(cfg: Config) -> Result<TeraLayer, Error> {
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

    Ok(TeraLayer {
        tera,
        default_context: ctx,
    })
}

// TODO remove, test function to see if this compiles when adding it as an extenstion
async fn after_routes(cfg: Config, router: axum::Router) -> Result<axum::Router, ()> {
    println!("loading locales");
    let st = load_locales(cfg).await;
    println!("locales ready");
    Ok(router.layer(Extension(Arc::new(st))))
}

// Tests for TeraLayer
// reading from configs, a test folder with templates and locales
// and printing some templates
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

        let mut locales = load_locales(cfg.clone()).await.unwrap();
        serde_json::json!({"lang": "zh-CN"});
        locales.default_context.insert("lang", &"de-DE");
        println!("{:?}", locales.default_context);
        let string = locales
            .tera
            .render_str(
                r#"{{ fluent(key="hello-world",lang=lang) }}"#,
                &locales.default_context,
            )
            .unwrap();
        assert_eq!(string, "Hallo Welt!");

        let mut context = tera::Context::new();
        context.insert("message", &"hello");
        let template = locales.tera.render("test.html", &context).unwrap();

        assert_eq!("<p>hello</p>", template.trim());
    }
}
