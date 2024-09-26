//! Configurable and Flexible CORS Middleware
//!
//! This middleware enables Cross-Origin Resource Sharing (CORS) by allowing
//! configurable origins, methods, and headers in HTTP requests. It can be tailored
//! to fit various application requirements, supporting permissive CORS or
//! specific rules as defined in the middleware configuration.

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};
use axum::Router as AXRouter;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower_http::cors;

/// CORS middleware configuration
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Cors {
    pub enable: bool,
    /// Allow origins
    pub allow_origins: Option<Vec<String>>,
    /// Allow headers
    pub allow_headers: Option<Vec<String>>,
    /// Allow methods
    pub allow_methods: Option<Vec<String>>,
    /// Max age
    pub max_age: Option<u64>,
}

impl Cors {
    /// Creates cors layer
    ///
    /// # Errors
    ///
    /// This function returns an error in the following cases:
    ///
    /// - If any of the provided origins in `allow_origins` cannot be parsed as a valid URI,
    ///   the function will return a parsing error.
    /// - If any of the provided headers in `allow_headers` cannot be parsed as valid HTTP headers,
    ///   the function will return a parsing error.
    /// - If any of the provided methods in `allow_methods` cannot be parsed as valid HTTP methods,
    ///   the function will return a parsing error.
    ///
    /// In all of these cases, the error returned will be the result of the `parse` method
    /// of the corresponding type.
    pub fn cors(&self) -> Result<cors::CorsLayer> {
        let mut cors: cors::CorsLayer = cors::CorsLayer::permissive();
        if let Some(allow_origins) = &self.allow_origins {
            // testing CORS, assuming https://example.com in the allow list:
            // $ curl -v --request OPTIONS 'localhost:5150/api/_ping' -H 'Origin: https://example.com' -H 'Acces
            // look for '< access-control-allow-origin: https://example.com' in response.
            // if it doesn't appear (test with a bogus domain), it is not allowed.
            let mut list = vec![];
            for origins in allow_origins {
                list.push(origins.parse()?);
            }
            cors = cors.allow_origin(list);
        }
        if let Some(allow_headers) = &self.allow_headers {
            let mut headers = vec![];
            for header in allow_headers {
                headers.push(header.parse()?);
            }
            cors = cors.allow_headers(headers);
        }
        if let Some(allow_methods) = &self.allow_methods {
            let mut methods = vec![];
            for method in allow_methods {
                methods.push(method.parse()?);
            }
            cors = cors.allow_methods(methods);
        }
        if let Some(max_age) = self.max_age {
            cors = cors.max_age(Duration::from_secs(max_age));
        }
        Ok(cors)
    }
}
impl MiddlewareLayer for Cors {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "cors"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.enable
    }

    /// Applies the CORS middleware layer to the Axum router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(self.cors()?))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::tests_cfg;
    use axum::{
        body::Body,
        http::{Method, Request},
        routing::get,
        Router,
    };
    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use tower::ServiceExt;

    #[rstest]
    #[case("default", None, None, None)]
    #[case("with_allow_headers", Some(vec!["token".to_string(), "user".to_string()]), None, None)]
    #[case("with_allow_methods", None, Some(vec!["post".to_string(), "get".to_string()]), None)]
    #[case("with_max_age", None, None, Some(20))]
    #[case("default", None, None, None)]
    #[tokio::test]
    async fn cors_enabled(
        #[case] test_name: &str,
        #[case] allow_headers: Option<Vec<String>>,
        #[case] allow_methods: Option<Vec<String>>,
        #[case] max_age: Option<u64>,
    ) {
        let middleware = Cors {
            enable: true,
            allow_origins: None,
            allow_headers,
            allow_methods,
            max_age,
        };

        let app = Router::new().route("/", get(|| async {}));
        let app = middleware
            .apply(app)
            .expect("apply middleware")
            .with_state(tests_cfg::app::get_app_context().await);

        let req = Request::builder()
            .uri("/")
            .method(Method::GET)
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(req).await.expect("valid response");

        assert_debug_snapshot!(
            format!("cors_[{test_name}]"),
            (
                format!(
                    "access-control-allow-origin: {:?}",
                    response.headers().get("access-control-allow-origin")
                ),
                format!("vary: {:?}", response.headers().get("vary")),
                format!(
                    "access-control-allow-methods: {:?}",
                    response.headers().get("access-control-allow-methods")
                ),
                format!(
                    "access-control-allow-headers: {:?}",
                    response.headers().get("access-control-allow-headers")
                ),
                format!("allow: {:?}", response.headers().get("allow")),
            )
        );
    }

    #[test]
    fn should_be_disabled() {
        let middleware = Cors {
            enable: false,
            allow_origins: None,
            allow_headers: None,
            allow_methods: None,
            max_age: None,
        };
        assert!(!middleware.is_enabled());
    }
}
