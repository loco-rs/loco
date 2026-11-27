//! JSON routes over HTML actions, like Rails' `/posts/1.json`.
//!
//! With this middleware on, the same actions answer as JSON when the path
//! asks for it:
//!
//! - a path ending in `.json` (`/posts/1.json` runs `/posts/1`), when
//!   `extension: true` is set (off by default), and
//! - a path under a configured prefix (`/api/v1/posts/1` runs `/posts/1`).
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
//! Other responses (JSON, files, CSV, `304 Not Modified`) pass through
//! unchanged, and headers such as `Set-Cookie` are kept.
//!
//! ```yaml
//! server:
//!   middlewares:
//!     respond_to:
//!       enable: true
//!       prefix: /api/v1
//!       extension: true
//!       sign_in_path: /session/new
//! ```
//!
//! Pick a prefix none of your routes use. Every request under it is routed
//! with the prefix removed, so a route that already lives there (the
//! starter app mounts its auth and home controllers under `/api`) can no
//! longer be reached with `prefix: /api`.
//!
//! This middleware wraps the whole app, so any middleware layered after it
//! sees the path as the client sent it (`/admin.json`, `/api/v1/admin`), not
//! the route that runs. Do not guard routes by path in such a middleware;
//! put the guard on the routes (`Routes::layer`) or in the handler.

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
    /// Serve every route as JSON under this path prefix too, e.g. `/api/v1`.
    /// The prefix is removed before routing, so it must not be a path your
    /// routes already use. A missing leading `/` is added.
    #[serde(default)]
    pub prefix: Option<String>,
    /// Answer as JSON when the path ends in `.json`, which is removed
    /// before routing. Off by default: with it on, every route also answers
    /// at `<path>.json`, so a path parameter can no longer end in `.json`
    /// (`/files/{name}` would see `report` for `/files/report.json`). Set
    /// `extension: true` to turn it on.
    #[serde(default)]
    pub extension: bool,
    /// A redirect to this path (your sign-in page), or to a path below it,
    /// answers `401` instead of `200 {"ok": true, ...}`. Only redirects on
    /// the same host count.
    #[serde(default)]
    pub sign_in_path: Option<String>,
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
        config: Config {
            prefix: config.prefix.as_deref().and_then(normalize_prefix),
            ..config.clone()
        },
        ctx: ctx.clone(),
    }
}

/// `api/v1/` and `/api/v1` both mean `/api/v1`; an empty prefix or `/` means
/// none.
fn normalize_prefix(prefix: &str) -> Option<String> {
    let trimmed = prefix.trim().trim_matches('/');
    (!trimmed.is_empty()).then(|| format!("/{trimmed}"))
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
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| req.uri().authority().map(ToString::to_string));

    // A `.json` file served as-is (a static asset, say) must still be
    // found, so a GET that 404s with the extension removed is tried again
    // unchanged. A JSON 404 came from a handler that ran and answered, so
    // it is not retried, and the retry only wins when it finds something.
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
        && !is_json(res.headers())
        && let Some(again) = retry
    {
        let original = call(inner, again).await;
        if original.status().is_success() || original.status() == StatusCode::NOT_MODIFIED {
            return original;
        }
    }
    to_json(config, host.as_deref(), res)
}

fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"))
}

async fn call(inner: axum::Router, req: Request) -> Response {
    match inner.oneshot(req).await {
        Ok(res) => res,
        Err(never) => match never {},
    }
}

/// Turns a browser-only response into JSON. Everything else is unchanged.
/// `host` is the request's `Host`, to tell a redirect on this site from one
/// to another.
fn to_json(config: &Config, host: Option<&str>, res: Response) -> Response {
    let status = res.status();
    let headers = res.headers();

    if status.is_redirection() {
        // Only real redirects with somewhere to go. `304 Not Modified` (from
        // the `etag` middleware) and `300 Multiple Choices` pass through.
        let to = headers
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let Some(to) = to.filter(|_| {
            matches!(
                status,
                StatusCode::MOVED_PERMANENTLY
                    | StatusCode::FOUND
                    | StatusCode::SEE_OTHER
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::PERMANENT_REDIRECT
            )
        }) else {
            return res;
        };
        let signed_out = config.sign_in_path.as_deref().is_some_and(|sign_in| {
            same_site_path(&to, host).is_some_and(|path| is_under(path, sign_in))
        });
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

/// The path part of a `Location` that points at this site: a relative
/// path, or a full URL whose host is `host`. `None` for another site.
fn same_site_path<'a>(to: &'a str, host: Option<&str>) -> Option<&'a str> {
    let after_scheme = to
        .split_once("://")
        .map(|(_, rest)| rest)
        .or_else(|| to.strip_prefix("//"));
    let Some(rest) = after_scheme else {
        return Some(to);
    };
    let (authority, path) = rest
        .find(['/', '?', '#'])
        .map_or((rest, "/"), |i| (&rest[..i], &rest[i..]));
    host.is_some_and(|host| authority.eq_ignore_ascii_case(host))
        .then_some(path)
}

/// `path` is `base` itself or below it: `/login`, `/login/`, `/login?x=1`
/// are under `/login`; `/login-help` is not.
fn is_under(path: &str, base: &str) -> bool {
    let base = base.trim_end_matches('/');
    path.strip_prefix(base)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '?', '#']))
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
        extract::Path,
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
    fn extension_is_off_unless_configured() {
        let c: Config = serde_yaml::from_str("enable: true\nprefix: /api").unwrap();
        assert!(!c.extension);
        assert_eq!(json_path(&c, "/posts/1.json"), None);
        assert_eq!(
            json_path(&c, "/api/posts/1"),
            Some(("/posts/1".to_string(), Via::Prefix))
        );

        let c: Config = serde_yaml::from_str("enable: true\nextension: true").unwrap();
        assert_eq!(
            json_path(&c, "/posts/1.json"),
            Some(("/posts/1".to_string(), Via::Extension))
        );
    }

    #[test]
    fn takes_path_from_same_site_location() {
        let host = Some("example.com");
        assert_eq!(same_site_path("/login?x=1", host), Some("/login?x=1"));
        assert_eq!(
            same_site_path("https://example.com/login", host),
            Some("/login")
        );
        assert_eq!(same_site_path("https://EXAMPLE.com", host), Some("/"));
        assert_eq!(same_site_path("https://other.com/login", host), None);
        assert_eq!(same_site_path("//other.com/login", host), None);
        assert_eq!(same_site_path("https://example.com/login", None), None);
    }

    #[test]
    fn sign_in_matches_the_path_not_a_prefix_of_it() {
        assert!(is_under("/login", "/login"));
        assert!(is_under("/login/", "/login"));
        assert!(is_under("/login?next=/x", "/login"));
        assert!(is_under("/login/sso", "/login/"));
        assert!(!is_under("/login-help", "/login"));
        assert!(!is_under("/log", "/login"));
    }

    #[tokio::test]
    async fn prefix_is_normalized() {
        let ctx = tests_cfg::app::get_app_context().await;
        for (given, want) in [
            ("api", Some("/api")),
            ("/api/", Some("/api")),
            ("api/v1/", Some("/api/v1")),
            ("/", None),
            ("", None),
        ] {
            let mw = new(
                &Config {
                    prefix: Some(given.to_string()),
                    ..config()
                },
                &ctx,
            );
            assert_eq!(mw.config.prefix.as_deref(), want, "{given}");
        }
    }

    #[test]
    fn defaults() {
        let c = Config::default();
        assert!(!c.enable);
        assert!(!c.extension, "`.json` handling is opt-in");
        assert!(c.prefix.is_none());
    }

    async fn show(Format(respond_to): Format) -> crate::Result<axum::response::Response> {
        format::respond_to(&respond_to)
            .html(|| format::html("<p>post</p>"))
            .json(|| format::json(json!({ "id": 1 })))
            .render()
    }

    async fn find(Path(id): Path<i32>, Format(respond_to): Format) -> crate::Result<Response> {
        if id != 1 {
            return Err(crate::Error::NotFound);
        }
        format::respond_to(&respond_to)
            .html(|| format::html("<p>record</p>"))
            .json(|| format::json(json!({ "id": id })))
            .render()
    }

    async fn app() -> axum::Router {
        app_with(config()).await
    }

    async fn app_with(config: Config) -> axum::Router {
        let ctx = tests_cfg::app::get_app_context().await;
        let router = AXRouter::new()
            .route("/posts/{id}", get(show))
            .route("/records/{id}", get(find))
            .route(
                "/cached",
                get(|| async { ([(header::ETAG, "\"v1\"")], format::json(json!({ "v": 1 }))) }),
            )
            .route(
                "/signin",
                axum::routing::post(|| async {
                    (
                        [(header::SET_COOKIE, "session=abc; HttpOnly")],
                        format::redirect("/posts/1"),
                    )
                }),
            )
            .route(
                "/elsewhere",
                get(|| async { format::redirect("https://other.com/login") }),
            )
            .route("/help", get(|| async { format::redirect("/login-help") }))
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
        // `etag` sits inside, as in the default stack.
        let router = crate::controller::middleware::etag::Etag { enable: true }
            .apply(router)
            .unwrap();
        new(&config, &ctx).apply(router).unwrap().with_state(ctx)
    }

    async fn send(method: &str, uri: &str, accept: &str) -> (StatusCode, String) {
        let (status, _, body) =
            send_with(app().await, method, uri, &[(header::ACCEPT, accept)]).await;
        (status, body)
    }

    async fn send_with(
        app: axum::Router,
        method: &str,
        uri: &str,
        headers: &[(header::HeaderName, &str)],
    ) -> (StatusCode, HeaderMap, String) {
        let mut req = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            req = req.header(name, *value);
        }
        let res = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, headers, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn not_modified_passes_through() {
        let (status, headers, body) = send_with(
            app().await,
            "GET",
            "/api/cached",
            &[(header::IF_NONE_MATCH, "\"v1\"")],
        )
        .await;
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert_eq!(headers.get(header::ETAG).unwrap(), "\"v1\"");
        assert_eq!(body, "");
    }

    #[tokio::test]
    async fn handler_not_found_is_not_retried() {
        // `/records/999.json` with the extension kept would be a bad `{id}`
        // (a 400); the handler's own JSON 404 must win.
        assert_eq!(
            send("GET", "/records/999.json", "*/*").await,
            (
                StatusCode::NOT_FOUND,
                r#"{"error":"not_found","description":"Resource was not found"}"#.to_string()
            )
        );
        assert_eq!(
            send("GET", "/records/1.json", "*/*").await,
            (StatusCode::OK, r#"{"id":1}"#.to_string())
        );
    }

    #[tokio::test]
    async fn converted_redirect_keeps_set_cookie() {
        let (status, headers, body) = send_with(app().await, "POST", "/api/signin", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"ok":true,"redirect":"/posts/1"}"#);
        assert_eq!(
            headers.get(header::SET_COOKIE).unwrap(),
            "session=abc; HttpOnly"
        );
        assert!(headers.get(header::LOCATION).is_none());
    }

    #[tokio::test]
    async fn only_same_site_sign_in_redirects_are_signed_out() {
        for path in ["/api/elsewhere", "/api/help"] {
            let (status, _, _) =
                send_with(app().await, "GET", path, &[(header::HOST, "example.com")]).await;
            assert_eq!(status, StatusCode::OK, "{path}");
        }
        let (status, _, _) = send_with(
            app().await,
            "GET",
            "/api/private",
            &[(header::HOST, "example.com")],
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn prefix_without_leading_slash_works() {
        let app = app_with(Config {
            prefix: Some("api/".to_string()),
            ..config()
        })
        .await;
        let (status, _, body) = send_with(app, "GET", "/api/posts/1", &[]).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, r#"{"id":1}"#));
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
