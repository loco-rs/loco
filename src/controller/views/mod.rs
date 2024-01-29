use axum::{async_trait, extract::FromRequestParts, http::request::Parts, Extension};
use serde::Serialize;

use crate::Result;

#[cfg(feature = "with-db")]
pub mod pagination;

pub trait TemplateEngine {
    /// Render a template located by `key`
    ///
    /// # Errors
    ///
    /// This function will return an error if render fails
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String>;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Engine<E>(pub E);

impl<E> Engine<E> {
    /// Creates a new [`Engine`] that wraps the given engine
    pub fn new(engine: E) -> Self {
        Self(engine)
    }
}

impl<E> From<E> for Engine<E> {
    fn from(inner: E) -> Self {
        Self::new(inner)
    }
}

#[async_trait]
impl<S, E> FromRequestParts<S> for Engine<E>
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
