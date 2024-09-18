//! Sets secure headers for your backend to promote security-by-default.
use std::{
    collections::{BTreeMap, HashMap},
    task::{Context, Poll},
};

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request},
    response::Response,
};
use futures_util::future::BoxFuture;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json;
use tower::{Layer, Service};

use crate::{Error, Result};

///
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
/// For the list of presets and their content look at [secure_headers.json](https://github.com/loco-rs/loco/blob/master/src/controller/middleware/secure_headers.rs)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecureHeadersConfig {
    pub preset: Option<String>,
    pub overrides: Option<BTreeMap<String, String>>,
}

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

impl Default for SecureHeadersConfig {
    fn default() -> Self {
        Self {
            preset: Some("github".to_string()),
            overrides: None,
        }
    }
}

impl SecureHeadersConfig {
    fn as_headers(&self) -> Result<Vec<(HeaderName, HeaderValue)>> {
        let mut headers = vec![];
        if let Some(preset) = &self.preset {
            let p = PRESETS.get(preset).ok_or_else(|| {
                Error::Message(format!(
                    "secure_headers: a preset named `{preset}` does not exist"
                ))
            })?;
            push_headers(&mut headers, p)?;
        }
        if let Some(overrides) = &self.overrides {
            push_headers(&mut headers, overrides)?;
        }
        Ok(headers)
    }
}

lazy_static! {
    static ref PRESETS: HashMap<String, BTreeMap<String, String>> =
        serde_json::from_str(include_str!("secure_headers.json")).unwrap();
}

/// The secure headers layer
#[derive(Clone)]
pub struct SecureHeaders {
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl SecureHeaders {
    /// Returns new secure headers middleware
    ///
    /// # Errors
    /// Fails if invalid header values found
    pub fn new(config: &SecureHeadersConfig) -> Result<Self> {
        Ok(Self {
            headers: config.as_headers()?,
        })
    }
}

impl<S> Layer<S> for SecureHeaders {
    type Service = SecureHeadersMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecureHeadersMiddleware {
            inner,
            layer: self.clone(),
        }
    }
}

/// The secure headers middleware
#[derive(Clone)]
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

    use axum::{routing::get, Router};
    use hyper::Method;
    use insta::assert_debug_snapshot;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn can_set_headers() {
        let config = SecureHeadersConfig {
            preset: Some("github".to_string()),
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
        assert_debug_snapshot!(response.headers());
    }

    #[tokio::test]
    async fn can_override_headers() {
        let mut overrides = BTreeMap::new();
        overrides.insert("X-Download-Options".to_string(), "foobar".to_string());
        overrides.insert("New-Header".to_string(), "baz".to_string());

        let config = SecureHeadersConfig {
            preset: Some("github".to_string()),
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
        assert_debug_snapshot!(response.headers());
    }

    #[tokio::test]
    async fn default_is_github_preset() {
        let config = SecureHeadersConfig::default();
        let app = Router::new()
            .route("/", get(|| async {}))
            .layer(SecureHeaders::new(&config).unwrap());

        let req = Request::builder()
            .uri("/")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_debug_snapshot!(response.headers());
    }
}
