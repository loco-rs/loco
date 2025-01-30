//! Sets secure headers for your backend to promote security-by-default.
//!
//! This middleware applies secure HTTP headers, providing pre-defined presets
//! (e.g., "github") and the ability to override or define custom headers.

use std::{
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request},
    response::Response,
    Router as AXRouter,
};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use tower::{Layer, Service};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Error, Result};

static PRESETS: OnceLock<HashMap<String, BTreeMap<String, String>>> = OnceLock::new();
fn get_presets() -> &'static HashMap<String, BTreeMap<String, String>> {
    PRESETS.get_or_init(|| {
        let json_data = include_str!("secure_headers.json");
        serde_json::from_str(json_data).unwrap()
    })
}
/// Sets a predefined or custom set of secure headers.
///
/// We recommend our `github` preset. Presets values are derived
/// from the [secure_headers](https://github.com/github/secure_headers) Ruby
/// library which Github (and originally Twitter) use.
///
/// To use a preset, in your `config/development.yaml`:
///
/// ```yaml
/// middlewares:
///   secure_headers:
///     preset: github
/// ```
///
/// You can also override individual headers on a given preset:
///
/// ```yaml
/// middlewares:
///   secure_headers:
///     preset: github
///     overrides:
///       foo: bar
/// ```
///
/// Or start from scratch:
///
///```yaml
/// middlewares:
///   secure_headers:
///     preset: empty
///     overrides:
///       one: two
/// ```
///
/// To support `htmx`, You can add the following override, to allow some inline
/// running of scripts:
///
/// ```yaml
/// secure_headers:
///     preset: github
///     overrides:
///         # this allows you to use HTMX, and has unsafe-inline. Remove or consider in production
///         "Content-Security-Policy": "default-src 'self' https:; font-src 'self' https: data:; img-src 'self' https: data:; object-src 'none'; script-src 'unsafe-inline' 'self' https:; style-src 'self' https: 'unsafe-inline'"
/// ```
///
/// For the list of presets and their content look at [secure_headers.json](https://github.com/loco-rs/loco/blob/master/src/controller/middleware/secure_headers.rs)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecureHeader {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default)]
    pub overrides: Option<BTreeMap<String, String>>,
}

impl Default for SecureHeader {
    fn default() -> Self {
        serde_json::from_value(json!({})).unwrap()
    }
}

fn default_preset() -> String {
    "github".to_string()
}

impl MiddlewareLayer for SecureHeader {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "secure_headers"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the secure headers layer to the application router
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(SecureHeaders::new(self)?))
    }
}

impl SecureHeader {
    /// Converts the configuration into a list of headers.
    ///
    /// Applies the preset headers and any custom overrides.
    fn as_headers(&self) -> Result<Vec<(HeaderName, HeaderValue)>> {
        let mut headers = vec![];

        let preset = &self.preset;
        let p = get_presets().get(preset).ok_or_else(|| {
            Error::Message(format!(
                "secure_headers: a preset named `{preset}` does not exist"
            ))
        })?;

        Self::push_headers(&mut headers, p)?;
        if let Some(overrides) = &self.overrides {
            Self::push_headers(&mut headers, overrides)?;
        }
        Ok(headers)
    }

    /// Helper function to push headers into a mutable vector.
    ///
    /// This function takes a map of header names and values, converting them
    /// into valid HTTP headers and adding them to the provided `headers`
    /// vector.
    fn push_headers(
        headers: &mut Vec<(HeaderName, HeaderValue)>,
        hm: &BTreeMap<String, String>,
    ) -> Result<()> {
        for (k, v) in hm {
            headers.push((
                HeaderName::from_bytes(k.clone().as_bytes()).map_err(Box::from)?,
                HeaderValue::from_str(v.clone().as_str()).map_err(Box::from)?,
            ));
        }
        Ok(())
    }
}

/// The [`SecureHeaders`] layer which wraps around the service and injects
/// security headers
#[derive(Clone, Debug)]
pub struct SecureHeaders {
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl SecureHeaders {
    /// Creates a new [`SecureHeaders`] instance with the provided
    /// configuration.
    ///
    /// # Errors
    /// Returns an error if any header values are invalid.
    pub fn new(config: &SecureHeader) -> Result<Self> {
        Ok(Self {
            headers: config.as_headers()?,
        })
    }
}

impl<S> Layer<S> for SecureHeaders {
    type Service = SecureHeadersMiddleware<S>;

    /// Wraps the provided service with the secure headers middleware.
    fn layer(&self, inner: S) -> Self::Service {
        SecureHeadersMiddleware {
            inner,
            layer: self.clone(),
        }
    }
}

/// The secure headers middleware
#[derive(Clone, Debug)]
#[must_use]
pub struct SecureHeadersMiddleware<S> {
    inner: S,
    layer: SecureHeaders,
}

impl<S> Service<Request<Body>> for SecureHeadersMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let layer = self.layer.clone();
        let future = self.inner.call(request);
        Box::pin(async move {
            let mut response: Response = future.await?;
            let headers = response.headers_mut();
            for (k, v) in &layer.headers {
                headers.insert(k, v.clone());
            }
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {

    use axum::{
        http::{HeaderMap, Method},
        routing::get,
        Router,
    };
    use insta::assert_debug_snapshot;
    use tower::ServiceExt;

    use super::*;
    fn normalize_headers(headers: &HeaderMap) -> BTreeMap<String, String> {
        headers
            .iter()
            .map(|(k, v)| {
                let key = k.to_string();
                let value = v.to_str().unwrap_or("").to_string();
                (key, value)
            })
            .collect()
    }
    #[tokio::test]
    async fn can_set_headers() {
        let config = SecureHeader {
            enable: true,
            preset: "github".to_string(),
            overrides: None,
        };
        let app = Router::new()
            .route("/", get(|| async {}))
            .layer(SecureHeaders::new(&config).unwrap());

        let req = Request::builder()
            .uri("/")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_debug_snapshot!(normalize_headers(response.headers()));
    }

    #[tokio::test]
    async fn can_override_headers() {
        let mut overrides = BTreeMap::new();
        overrides.insert("X-Download-Options".to_string(), "foobar".to_string());
        overrides.insert("New-Header".to_string(), "baz".to_string());

        let config = SecureHeader {
            enable: true,
            preset: "github".to_string(),
            overrides: Some(overrides),
        };
        let app = Router::new()
            .route("/", get(|| async {}))
            .layer(SecureHeaders::new(&config).unwrap());

        let req = Request::builder()
            .uri("/")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_debug_snapshot!(normalize_headers(response.headers()));
    }

    #[tokio::test]
    async fn default_is_github_preset() {
        let config = SecureHeader::default();
        let app = Router::new()
            .route("/", get(|| async {}))
            .layer(SecureHeaders::new(&config).unwrap());

        let req = Request::builder()
            .uri("/")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_debug_snapshot!(normalize_headers(response.headers()));
    }
}
