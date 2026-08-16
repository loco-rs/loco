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
    /// Creates a new [`ViewEngine`] that wraps the given engine
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
    let tera = crate::tera::instance();
    // Not autoescaped: this renders an arbitrary caller-supplied string with no
    // filename to infer a content type from, matching what Tera 1 did here (its
    // `render_str` named the template `__tera_one_off`, which matched no
    // autoescape suffix). Callers emitting HTML must escape themselves.
    Ok(tera.render_str(template, &tera::Context::from_serialize(&data)?, false)?)
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
    type Rejection = crate::Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let Extension(tl): Extension<Self> = Extension::from_request_parts(parts, state)
            .await
            .map_err(|_| crate::Error::string("TeraLayer missing. Is the TeraLayer installed?"))?;

        Ok(tl)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn renders_inline_template() {
        assert_eq!(
            template("{{name}} website", json!({"name": "Loco"})).unwrap(),
            "Loco website"
        );
    }

    /// Pins the autoescape decision above. Tera 1 did not escape here either, so
    /// changing this would silently alter output for every existing caller.
    #[test]
    fn inline_templates_are_not_autoescaped() {
        assert_eq!(
            template("{{ html }}", json!({"html": "<b>hi</b>"})).unwrap(),
            "<b>hi</b>"
        );
    }

    /// Built-in filters and `get_env` come from the shared Tera factory, so they
    /// must be reachable from inline templates too.
    #[test]
    fn inline_templates_see_builtin_filters_and_get_env() {
        assert_eq!(
            template("{{ n | number_with_delimiter }}", json!({"n": 12_345})).unwrap(),
            "12,345"
        );
        assert_eq!(
            template(
                r#"{{ get_env(name="LOCO_INLINE_NOPE", default="fallback") }}"#,
                json!({})
            )
            .unwrap(),
            "fallback"
        );
    }
}
