//! [Initializer] to add a [NormalizePathLayer] middleware to handle a trailing
//! `/` at the end of URIs.
//!
//! See the [layer's docs][normalize-docs] for more details.
//!
//! Note that the normal approach to adding middleware via [Router::layer]
//! results in the middleware running after routing has already occurred. This
//! means that any middleware that re-writes the request URI, including
//! [NormalizePathLayer], will not work as expected if added using
//! [Router::layer]. As a workaround, the middleware can be added by wrapping
//! the entire router. See [axum's docs][axum-docs] for more details and an
//! example.
//!
//! [normalize-docs]: https://docs.rs/tower-http/latest/tower_http/normalize_path/index.html
//! [axum-docs]: https://docs.rs/axum/latest/axum/middleware/index.html#rewriting-request-uri-in-middleware
use async_trait::async_trait;
use axum::Router;
use loco_rs::prelude::*;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;

#[allow(clippy::module_name_repetitions)]
pub struct NormalizePathInitializer;

#[async_trait]
impl Initializer for NormalizePathInitializer {
    fn name(&self) -> String {
        "normalize-path".to_string()
    }

    async fn after_routes(&self, router: Router, _ctx: &AppContext) -> Result<Router> {
        let router = NormalizePathLayer::trim_trailing_slash().layer(router);
        let router = Router::new().nest_service("", router);
        Ok(router)
    }
}
