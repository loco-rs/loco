// Choose the correct engine implementation based on the feature flag
#[cfg(feature = "embedded_assets")]
pub mod engine_embedded;
#[cfg(feature = "embedded_assets")]
pub use engine_embedded as engines;

#[cfg(not(feature = "embedded_assets"))]
pub mod engine;
#[cfg(not(feature = "embedded_assets"))]
pub use engine as engines;

use axum::{extract::FromRequestParts, http::request::Parts, Extension};
use serde::Serialize;
pub mod tera_builtins;
use crate::Result;

#[cfg(feature = "with-db")]
pub mod pagination;

pub trait ViewRenderer {
    /// Render a view template located by `key`
    ///
    /// # Errors
    ///
    /// This function will return an error if render fails
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String>;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ViewEngine<E>(pub E);

impl<E> ViewEngine<E> {
    /// Creates a new [`Engine`] that wraps the given engine
    pub fn new(engine: E) -> Self {
        Self(engine)
    }
}

/// A struct representing an inline Tera view renderer.
///
/// This struct provides functionality to render templates using the Tera
/// templating engine directly from raw template strings.
///
/// # Example
/// ```
/// use serde_json::json;
/// use loco_rs::controller::views;
/// let render = views::template("{{name}} website", json!({"name": "Loco"})).unwrap();
/// assert_eq!(render, "Loco website");
/// ```
///
/// # Errors
///
/// This function will return an error if building fails
pub fn template<S>(template: &str, data: S) -> Result<String>
where
    S: Serialize,
{
    let mut tera = tera::Tera::default();
    Ok(tera.render_str(template, &tera::Context::from_serialize(data)?)?)
}

impl<E> From<E> for ViewEngine<E> {
    fn from(inner: E) -> Self {
        Self::new(inner)
    }
}

impl<S, E> FromRequestParts<S> for ViewEngine<E>
where
    S: Send + Sync,
    E: Clone + Send + Sync + 'static,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let Extension(tl): Extension<Self> = Extension::from_request_parts(parts, state)
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
