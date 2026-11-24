//! JSON routes over HTML actions, like Rails' `/posts/1.json`.
//!
//! With this middleware on, the same actions answer as JSON when the path
//! asks for it:
//!
//! - a path ending in `.json` (`/posts/1.json` runs `/posts/1`), and
//! - a path under a configured prefix (`/api/posts/1` runs `/posts/1`).
//!
//! The extension or prefix is removed before routing and the request is
//! marked as wanting JSON, so the [`Format`](super::format::Format)
//! extractor returns [`RespondTo::Json`] and
//! [`format::respond_to`](crate::controller::format::respond_to) picks the
//! JSON branch. Responses that only make sense to a browser are turned into
//! JSON on the way out:
//!
//! - a redirect to `sign_in_path` becomes
//!   `401 {"error": "signed_out", "redirect": "<to>"}`;
//! - any other redirect becomes `200 {"ok": true, "redirect": "<to>"}`;
//! - an HTML error page becomes `{"error": "<reason>"}` with its status;
//! - an HTML page from an action with no JSON branch becomes
//!   `406 {"error": "not_acceptable"}`.
//!
//! Other responses (JSON, files, CSV) pass through unchanged, and headers
//! such as `Set-Cookie` are kept.
//!
//! ```yaml
//! server:
//!   middlewares:
//!     respond_to:
//!       enable: true
//!       prefix: /api
//!       extension: true
//!       sign_in_path: /session/new
//! ```

use std::{convert::Infallible, sync::Arc};

use axum::{
    body::Body,
    extract::Request,
    http::{header, uri::PathAndQuery, HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    Router as AXRouter,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::ServiceExt;

use super::format::RespondTo;
use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

/// `server.middlewares.respond_to` configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Turns the middleware on. Off by default.
    #[serde(default)]
    pub enable: bool,
    /// Serve every route as JSON under this path prefix too, e.g. `/api` or
    /// `/api/v1`. The prefix is removed before routing.
    #[serde(default)]
    pub prefix: Option<String>,
    /// Answer as JSON when the path ends in `.json`, which is removed
    /// before routing. On by default.
    #[serde(default = "default_extension")]
    pub extension: bool,
    /// A redirect to a path starting with this (your sign-in page) answers
    /// `401` instead of `200 {"ok": true, ...}`.
    #[serde(default)]
    pub sign_in_path: Option<String>,
}

const fn default_extension() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        super::defaults()
    }
}

/// The `respond_to` middleware. Build it with [`new`].
pub struct Middleware {
    config: Config,
    ctx: AppContext,
}

/// Creates the middleware. It needs the [`AppContext`] because it routes
/// the rewritten request through the finished app.
#[must_use]
pub fn new(config: &Config, ctx: &AppContext) -> Middleware {
    Middleware {
        config: config.clone(),
        ctx: ctx.clone(),
    }
}

impl MiddlewareLayer for Middleware {
    fn name(&self) -> &'static str {
        "respond_to"
    }

    fn is_enabled(&self) -> bool {
        self.config.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(&self.config)
    }

    /// Wraps the whole app, so the request can be routed again with the
    /// extension or prefix removed. Middlewares applied after this one see
    /// the original request; it must come after `fallback` and `static`,
    /// which set the app's fallback.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        let inner = app.with_state(self.ctx.clone());
        let config = Arc::new(self.config.clone());
        Ok(
            AXRouter::new().fallback_service(tower::service_fn(move |req: Request| {
                let inner = inner.clone();
                let config = config.clone();
                async move { Ok::<_, Infallible>(handle(&config, inner, req).await) }
            })),
        )
    }
}

/// How a request asked for JSON.
#[derive(Debug, PartialEq, Eq)]
enum Via {
    Prefix,
    Extension,
}

/// The path to route instead, when the request asks for JSON by its path.
fn json_path(config: &Config, path: &str) -> Option<(String, Via)> {
    let under_prefix = config
        .prefix
        .as_deref()
        .map(|p| p.trim_end_matches('/'))
        .filter(|p| !p.is_empty())
        .and_then(|prefix| path.strip_prefix(prefix))
        .filter(|after| after.is_empty() || after.starts_with('/'));
    let (mut rest, mut via) = under_prefix.map_or((path, None), |after| (after, Some(Via::Prefix)));
    if config.extension
        && let Some(stem) = rest.strip_suffix(".json")
        && !stem.is_empty()
        && !stem.ends_with('/')
    {
        rest = stem;
        via.get_or_insert(Via::Extension);
    }
    let rest = if rest.is_empty() { "/" } else { rest };
    via.map(|via| (rest.to_string(), via))
}

fn with_path(uri: &Uri, path: &str) -> Uri {
    let path_and_query = uri
        .query()
        .map_or_else(|| path.to_string(), |query| format!("{path}?{query}"));
    let mut parts = uri.clone().into_parts();
    parts.path_and_query = PathAndQuery::try_from(path_and_query).ok();
    Uri::from_parts(parts).unwrap_or_else(|_| uri.clone())
}

async fn handle(config: &Config, inner: axum::Router, req: Request) -> Response {
    let Some((path, via)) = json_path(config, req.uri().path()) else {
        return call(inner, req).await;
    };

    // A `.json` file served as-is (a static asset, say) must still be
    // found, so a GET that 404s with the extension removed is tried again
    // unchanged.
    let retry = (via == Via::Extension && matches!(*req.method(), Method::GET | Method::HEAD))
        .then(|| {
            let mut again = Request::new(Body::empty());
            *again.method_mut() = req.method().clone();
            *again.uri_mut() = req.uri().clone();
            *again.version_mut() = req.version();
            *again.headers_mut() = req.headers().clone();
            *again.extensions_mut() = req.extensions().clone();
            again
        });

    let (mut parts, body) = req.into_parts();
    parts.uri = with_path(&parts.uri, &path);
    parts.headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json"),
    );
    parts.extensions.insert(RespondTo::Json);
    let res = call(inner.clone(), Request::from_parts(parts, body)).await;

    if res.status() == StatusCode::NOT_FOUND
        && let Some(again) = retry
    {
        let original = call(inner, again).await;
        if original.status() != StatusCode::NOT_FOUND {
            return original;
        }
    }
    to_json(config, res)
}

async fn call(inner: axum::Router, req: Request) -> Response {
    match inner.oneshot(req).await {
        Ok(res) => res,
        Err(never) => match never {},
    }
}

/// Turns a browser-only response into JSON. Everything else is unchanged.
fn to_json(config: &Config, res: Response) -> Response {
    let status = res.status();
    let headers = res.headers();

    if status.is_redirection() {
        let to = headers
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("/")
            .to_string();
        let signed_out = config
            .sign_in_path
            .as_deref()
            .is_some_and(|sign_in| location_path(&to).starts_with(sign_in));
        let answer = if signed_out {
            (
                StatusCode::UNAUTHORIZED,
                json!({ "error": "signed_out", "redirect": to }),
            )
        } else {
            (StatusCode::OK, json!({ "ok": true, "redirect": to }))
        };
        return replace(res.headers(), answer);
    }

    let is_html = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/html"));
    if !is_html {
        return res;
    }
    if status.is_success() {
        return replace(
            res.headers(),
            (
                StatusCode::NOT_ACCEPTABLE,
                json!({
                    "error": "not_acceptable",
                    "description": "This action has no JSON response",
                }),
            ),
        );
    }
    let reason = match status {
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::UNAUTHORIZED => "unauthorized",
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => "bad_request",
        _ => "error",
    };
    replace(res.headers(), (status, json!({ "error": reason })))
}

/// The path part of a `Location`, which may be a full URL.
fn location_path(to: &str) -> &str {
    to.split_once("://")
        .map_or(to, |(_, rest)| rest.find('/').map_or("/", |i| &rest[i..]))
}

/// A JSON response that keeps the original's headers, except those that
/// described the old body.
fn replace(original: &HeaderMap, (status, body): (StatusCode, serde_json::Value)) -> Response {
    let mut res = (status, axum::Json(body)).into_response();
    for (name, value) in original {
        if [
            header::CONTENT_TYPE,
            header::CONTENT_LENGTH,
            header::CONTENT_ENCODING,
            header::LOCATION,
            header::ETAG,
            header::TRANSFER_ENCODING,
        ]
        .contains(name)
        {
            continue;
        }
        res.headers_mut().append(name, value.clone());
    }
    res
}

#[cfg(test)]
mod tests {
    use axum::{
        http::{header, StatusCode},
        response::Html,
        routing::get,
    };
    use tower::ServiceExt;

    use super::*;
    use crate::{
        controller::{format, middleware::format::Format},
        tests_cfg,
    };

    fn config() -> Config {
        Config {
            enable: true,
            prefix: Some("/api".to_string()),
            extension: true,
            sign_in_path: Some("/login".to_string()),
        }
    }

    #[test]
    fn finds_json_paths() {
        let c = config();
        assert_eq!(json_path(&c, "/posts"), None);
        assert_eq!(
            json_path(&c, "/posts/1.json"),
            Some(("/posts/1".to_string(), Via::Extension))
        );
        assert_eq!(
            json_path(&c, "/api/posts/1"),
            Some(("/posts/1".to_string(), Via::Prefix))
        );
        assert_eq!(
            json_path(&c, "/api/posts/1.json"),
            Some(("/posts/1".to_string(), Via::Prefix))
        );
        assert_eq!(json_path(&c, "/api"), Some(("/".to_string(), Via::Prefix)));
        assert_eq!(json_path(&c, "/apiary"), None);
        assert_eq!(json_path(&c, "/.json"), None);

        let no_ext = Config {
            extension: false,
            ..config()
        };
        assert_eq!(json_path(&no_ext, "/posts.json"), None);
    }

    #[test]
    fn takes_path_from_location() {
        assert_eq!(location_path("/login?x=1"), "/login?x=1");
        assert_eq!(location_path("https://example.com/login"), "/login");
        assert_eq!(location_path("https://example.com"), "/");
    }

    #[test]
    fn defaults() {
        let c = Config::default();
        assert!(!c.enable);
        assert!(c.extension);
        assert!(c.prefix.is_none());
    }

    async fn show(Format(respond_to): Format) -> crate::Result<axum::response::Response> {
        format::respond_to(&respond_to)
            .html(|| format::html("<p>post</p>"))
            .json(|| format::json(json!({ "id": 1 })))
            .render()
    }

    async fn app() -> axum::Router {
        let ctx = tests_cfg::app::get_app_context().await;
        let router = AXRouter::new()
            .route("/posts/{id}", get(show))
            .route("/page", get(|| async { Html("<p>html only</p>") }))
            .route(
                "/save",
                axum::routing::post(|| async { format::redirect("/posts/1") }),
            )
            .route("/private", get(|| async { format::redirect("/login") }))
            .route(
                "/files/data.json",
                get(|| async { format::json(json!({ "file": true })) }),
            )
            .fallback(|| async { (StatusCode::NOT_FOUND, Html("<h1>404</h1>")) });
        new(&config(), &ctx).apply(router).unwrap().with_state(ctx)
    }

    async fn send(method: &str, uri: &str, accept: &str) -> (StatusCode, String) {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::ACCEPT, accept)
            .body(Body::empty())
            .unwrap();
        let res = app().await.oneshot(req).await.unwrap();
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn same_action_answers_html_or_json() {
        assert_eq!(
            send("GET", "/posts/1", "text/html").await,
            (StatusCode::OK, "<p>post</p>".to_string())
        );
        assert_eq!(
            send("GET", "/posts/1.json", "text/html").await,
            (StatusCode::OK, r#"{"id":1}"#.to_string())
        );
        assert_eq!(
            send("GET", "/api/posts/1", "text/html").await,
            (StatusCode::OK, r#"{"id":1}"#.to_string())
        );
        assert_eq!(
            send("GET", "/posts/1", "application/json").await,
            (StatusCode::OK, r#"{"id":1}"#.to_string())
        );
    }

    #[tokio::test]
    async fn browser_responses_become_json() {
        assert_eq!(
            send("POST", "/api/save", "*/*").await,
            (
                StatusCode::OK,
                r#"{"ok":true,"redirect":"/posts/1"}"#.to_string()
            )
        );
        assert_eq!(
            send("GET", "/api/private", "*/*").await,
            (
                StatusCode::UNAUTHORIZED,
                r#"{"error":"signed_out","redirect":"/login"}"#.to_string()
            )
        );
        let (status, body) = send("GET", "/api/page", "*/*").await;
        assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
        assert!(body.contains("not_acceptable"));
        assert_eq!(
            send("GET", "/api/missing", "*/*").await,
            (
                StatusCode::NOT_FOUND,
                r#"{"error":"not_found"}"#.to_string()
            )
        );
    }

    #[tokio::test]
    async fn html_paths_are_untouched() {
        assert_eq!(
            send("GET", "/page", "text/html").await,
            (StatusCode::OK, "<p>html only</p>".to_string())
        );
        let (status, _) = send("POST", "/save", "text/html").await;
        assert_eq!(status, StatusCode::SEE_OTHER);
    }

    #[tokio::test]
    async fn real_json_files_still_served() {
        assert_eq!(
            send("GET", "/files/data.json", "*/*").await,
            (StatusCode::OK, r#"{"file":true}"#.to_string())
        );
    }
}
