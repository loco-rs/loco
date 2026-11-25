//! Single-page app (SPA) middleware.
//!
//! Serves a frontend build folder (for example Vite's `frontend/dist`) under a
//! mount path, alongside the normal `static` folder and your own routes.
//!
//! * A file that exists is served as is.
//! * A missing path that looks like a client-side route (`/app/posts/42`, no
//!   file extension) gets the build's `index.html`, so the app's own router
//!   picks the page after a hard refresh.
//! * A missing path that looks like a file (`/app/assets/old-abc123.js`) gets
//!   a plain `404`, never `index.html`: a stale bundle reference must fail
//!   loudly rather than be parsed as HTML.
//! * Only `GET` and `HEAD` are served; other methods answer `404`, so an
//!   unmatched `POST` is not swallowed by the app shell.
//! * A path whose last segment has a dot counts as a file, so a client route
//!   such as `/app/users/jane.doe` is a `404`, not the app shell.
//! * A directory is never listed or redirected; it is treated like a missing
//!   path, so `/app/docs` gets the app shell.
//!
//! Cache headers follow how bundlers name their output: files under
//! `assets/` (Vite's default output folder) carry a content hash in their
//! names, so they get a long, immutable cache. HTML (the entry point that
//! points at those hashes) and other files that keep their names
//! (`favicon.ico`, `robots.txt`) are always revalidated.
//!
//! ```yaml
//! server:
//!   middlewares:
//!     spa:
//!       enable: true
//!       uri: "/app"
//!       path: "frontend/dist"
//! ```

use std::path::{Path, PathBuf};

use axum::{
    body::Body,
    extract::Request,
    http::{
        header::{HeaderValue, CACHE_CONTROL, CONTENT_TYPE},
        Method, StatusCode,
    },
    response::{IntoResponse, Response},
    Router as AXRouter,
};
use serde::{Deserialize, Serialize};
use tower::Service;
use tower_http::services::{ServeDir, ServeFile};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Error, Result};

/// Cache-Control used for hashed build assets by default.
pub const IMMUTABLE: &str = "public, max-age=31536000, immutable";
/// Cache-Control used for HTML (the app shell) by default.
pub const NO_CACHE: &str = "no-cache";

/// SPA middleware configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Spa {
    #[serde(default)]
    pub enable: bool,
    /// Where the app is mounted. `/` serves it for every path no route
    /// matched.
    #[serde(default = "default_uri")]
    pub uri: String,
    /// The build folder on disk.
    #[serde(default = "default_path")]
    pub path: PathBuf,
    /// The app shell, relative to `path`. Served for client-side routes.
    #[serde(default = "default_index")]
    pub index: PathBuf,
    /// Refuse to start when `path` or `index` is missing.
    #[serde(default = "default_must_exist")]
    pub must_exist: bool,
    /// Serve `file.gz` when it exists and the client accepts gzip.
    #[serde(default)]
    pub precompressed: bool,
    /// Cache-Control values by response kind.
    #[serde(default = "default_cache_control")]
    pub cache_control: CacheControl,
}

/// Cache-Control values by response kind.
///
/// Set one to `~` (null) to send no header for that kind of response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheControl {
    /// For files under `assets_prefix` (hashed JS, CSS, images, fonts).
    #[serde(default = "default_assets_cache")]
    pub assets: Option<String>,
    /// The folder, relative to `path`, whose files carry a content hash in
    /// their names. An empty string counts every non-HTML file as hashed.
    #[serde(default = "default_assets_prefix")]
    pub assets_prefix: String,
    /// For HTML responses, including the `index.html` fallback.
    #[serde(default = "default_html_cache")]
    pub html: Option<String>,
    /// For other files, which keep their names between builds
    /// (`favicon.ico`, `robots.txt`, `manifest.json`).
    #[serde(default = "default_other_cache")]
    pub other: Option<String>,
}

impl Default for Spa {
    fn default() -> Self {
        super::defaults()
    }
}

impl Default for CacheControl {
    fn default() -> Self {
        super::defaults()
    }
}

fn default_uri() -> String {
    "/".to_string()
}

fn default_path() -> PathBuf {
    PathBuf::from("frontend").join("dist")
}

fn default_index() -> PathBuf {
    PathBuf::from("index.html")
}

const fn default_must_exist() -> bool {
    true
}

fn default_cache_control() -> CacheControl {
    CacheControl::default()
}

#[allow(clippy::unnecessary_wraps)]
fn default_assets_cache() -> Option<String> {
    Some(IMMUTABLE.to_string())
}

fn default_assets_prefix() -> String {
    "assets/".to_string()
}

#[allow(clippy::unnecessary_wraps)]
fn default_html_cache() -> Option<String> {
    Some(NO_CACHE.to_string())
}

#[allow(clippy::unnecessary_wraps)]
fn default_other_cache() -> Option<String> {
    Some(NO_CACHE.to_string())
}

/// `true` when the last path segment has no extension, i.e. the request is a
/// client-side route rather than a file.
fn is_client_route(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_none_or(|segment| Path::new(segment).extension().is_none())
}

fn is_html_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm"))
}

fn header_value(key: &str, value: Option<&String>) -> Result<Option<HeaderValue>> {
    value
        .map(|v| {
            HeaderValue::from_str(v).map_err(|_| {
                Error::Message(format!(
                    "spa: `cache_control.{key}` value `{v}` is not a valid header value"
                ))
            })
        })
        .transpose()
}

/// Turns the configured `uri` into a mount path: `/` or `/a/b` with no
/// trailing slash.
fn mount_path(uri: &str) -> Result<String> {
    let uri = uri.trim();
    if uri.is_empty() {
        return Ok("/".to_string());
    }
    if !uri.starts_with('/') {
        return Err(Error::Message(format!(
            "spa: `uri` must start with `/`, got `{uri}`"
        )));
    }
    let uri = uri.trim_end_matches('/');
    if uri.is_empty() {
        return Ok("/".to_string());
    }
    if uri.contains("//") {
        return Err(Error::Message(format!(
            "spa: `uri` must not contain an empty segment, got `{uri}`"
        )));
    }
    Ok(uri.to_string())
}

#[derive(Clone)]
struct SpaService {
    files: ServeDir,
    index: PathBuf,
    precompressed: bool,
    assets_prefix: String,
    assets_cache: Option<HeaderValue>,
    html_cache: Option<HeaderValue>,
    other_cache: Option<HeaderValue>,
}

impl SpaService {
    async fn serve(self, req: Request) -> Response {
        if req.method() != Method::GET && req.method() != Method::HEAD {
            return StatusCode::NOT_FOUND.into_response();
        }
        // Inside a nested mount this path is relative to `uri`.
        let path = req.uri().path().to_string();
        let client_route = is_client_route(&path);
        let (parts, body) = req.into_parts();
        let file_req = Request::from_parts(parts.clone(), body);

        let mut files = self.files.clone();
        let mut res = match files.call(file_req).await {
            Ok(res) => res.into_response(),
            Err(err) => match err {},
        };

        let mut shell = false;
        if res.status() == StatusCode::NOT_FOUND && client_route {
            shell = true;
            let mut index = ServeFile::new(&self.index);
            if self.precompressed {
                index = index.precompressed_gzip();
            }
            res = match index.call(Request::from_parts(parts, Body::empty())).await {
                Ok(res) => res.into_response(),
                Err(err) => match err {},
            };
        }

        self.set_cache_control(&mut res, &path, shell);
        res
    }

    /// Picks the header from the request as well as the response: a `304`
    /// carries no `Content-Type`, and must not turn the app shell into an
    /// immutable asset.
    fn set_cache_control(&self, res: &mut Response, path: &str, shell: bool) {
        let status = res.status();
        if !(status.is_success() || status == StatusCode::NOT_MODIFIED) {
            return;
        }
        let html = shell
            || is_html_path(path)
            || res
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.starts_with("text/html"));
        let value = if html {
            &self.html_cache
        } else if path
            .trim_start_matches('/')
            .starts_with(&self.assets_prefix)
        {
            &self.assets_cache
        } else {
            &self.other_cache
        };
        if let Some(value) = value {
            res.headers_mut().insert(CACHE_CONTROL, value.clone());
        }
    }
}

impl MiddlewareLayer for Spa {
    fn name(&self) -> &'static str {
        "spa"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Mounts the build folder at `uri`. Fails when `uri` does not start
    /// with `/`, when a `cache_control` value is not a valid header, or when
    /// `must_exist` is set and the folder or its index file is missing.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        let uri = mount_path(&self.uri)?;
        let cache = &self.cache_control;
        let assets_cache = header_value("assets", cache.assets.as_ref())?;
        let html_cache = header_value("html", cache.html.as_ref())?;
        let other_cache = header_value("other", cache.other.as_ref())?;
        let mut assets_prefix = cache.assets_prefix.trim_matches('/').to_string();
        if !assets_prefix.is_empty() {
            assets_prefix.push('/');
        }

        let index = self.path.join(&self.index);
        if self.must_exist && (!self.path.exists() || !index.exists()) {
            return Err(Error::Message(format!(
                "spa: build folder `{}` or its index `{}` not found; build the frontend first \
                 or set `must_exist: false`",
                self.path.display(),
                index.display(),
            )));
        }

        // A directory is a missing file: the shell is served for it below.
        // ServeDir's own handling would redirect `/docs` to `/docs/` using
        // the path with `uri` stripped, which points outside the mount.
        let mut files = ServeDir::new(&self.path).append_index_html_on_directories(false);
        if self.precompressed {
            files = files.precompressed_gzip();
        }
        let service = SpaService {
            files,
            index,
            precompressed: self.precompressed,
            assets_prefix,
            assets_cache,
            html_cache,
            other_cache,
        };
        let router = AXRouter::new().fallback(move |req: Request| service.clone().serve(req));

        if uri == "/" {
            Ok(app.fallback_service(router))
        } else {
            Ok(app.nest_service(&uri, router))
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        http::header::{ACCEPT_ENCODING, IF_MODIFIED_SINCE, LOCATION},
        routing::get,
    };
    use tower::ServiceExt;

    use super::*;

    fn build_folder() -> tree_fs::Tree {
        tree_fs::TreeBuilder::default()
            .drop(true)
            .add("dist/index.html", "<h1>shell</h1>")
            .add("dist/assets/app-abc123.js", "console.log(1)")
            .add("dist/robots.txt", "ok")
            .add("dist/docs/index.html", "<h1>docs</h1>")
            .create()
            .expect("create build folder")
    }

    fn spa(root: &Path, uri: &str) -> Spa {
        Spa {
            enable: true,
            uri: uri.to_string(),
            path: root.join("dist"),
            ..Default::default()
        }
    }

    async fn router(spa: &Spa) -> AXRouter {
        spa.apply(AXRouter::new())
            .expect("apply spa")
            .with_state(crate::tests_cfg::app::get_app_context().await)
    }

    async fn call(
        app: &AXRouter,
        method: Method,
        uri: &str,
    ) -> (StatusCode, Option<String>, String) {
        send(app, Request::builder().method(method).uri(uri)).await
    }

    async fn send(
        app: &AXRouter,
        req: axum::http::request::Builder,
    ) -> (StatusCode, Option<String>, String) {
        let res = app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let cache = res
            .headers()
            .get(CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_string());
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, cache, String::from_utf8_lossy(&body).to_string())
    }

    #[test]
    fn defaults() {
        let spa = Spa::default();
        assert!(!spa.enable);
        assert_eq!(spa.uri, "/");
        assert_eq!(spa.path, PathBuf::from("frontend/dist"));
        assert_eq!(spa.index, PathBuf::from("index.html"));
        assert!(spa.must_exist);
        assert!(!spa.precompressed);
        assert_eq!(spa.cache_control.assets.as_deref(), Some(IMMUTABLE));
        assert_eq!(spa.cache_control.assets_prefix, "assets/");
        assert_eq!(spa.cache_control.html.as_deref(), Some(NO_CACHE));
        assert_eq!(spa.cache_control.other.as_deref(), Some(NO_CACHE));
    }

    #[test]
    fn null_cache_control_disables_the_header() {
        let spa: Spa = serde_yaml::from_str("enable: true\ncache_control:\n  html: ~\n").unwrap();
        assert_eq!(spa.cache_control.assets.as_deref(), Some(IMMUTABLE));
        assert_eq!(spa.cache_control.html, None);
    }

    #[test]
    fn client_routes_have_no_extension() {
        assert!(is_client_route("/"));
        assert!(is_client_route("/posts/42"));
        assert!(is_client_route("/posts/"));
        assert!(!is_client_route("/assets/app.js"));
        assert!(!is_client_route("/favicon.ico"));
    }

    #[test]
    fn mount_paths_are_normalised() {
        assert_eq!(mount_path("").unwrap(), "/");
        assert_eq!(mount_path("  ").unwrap(), "/");
        assert_eq!(mount_path("/").unwrap(), "/");
        assert_eq!(mount_path("//").unwrap(), "/");
        assert_eq!(mount_path(" /app/ ").unwrap(), "/app");
        assert_eq!(mount_path("/a/b").unwrap(), "/a/b");
        assert!(mount_path("app").is_err());
        assert!(mount_path("/a//b").is_err());
    }

    #[test]
    fn bad_uri_is_an_error_not_a_panic() {
        let tree = build_folder();
        for uri in ["app", "/a//b"] {
            let err = spa(&tree.root, uri).apply(AXRouter::new()).unwrap_err();
            assert!(err.to_string().contains("`uri`"), "{uri}: {err}");
        }
        for uri in ["", "//", "/app/"] {
            assert!(spa(&tree.root, uri).apply(AXRouter::new()).is_ok(), "{uri}");
        }
    }

    #[test]
    fn invalid_cache_control_is_an_error() {
        let tree = build_folder();
        let mut spa = spa(&tree.root, "/app");
        spa.cache_control.html = Some("no-cache\n".to_string());
        let err = spa.apply(AXRouter::new()).unwrap_err();
        assert!(err.to_string().contains("cache_control.html"), "{err}");
    }

    #[test]
    fn must_exist_refuses_a_missing_build() {
        let spa = Spa {
            enable: true,
            path: PathBuf::from("does/not/exist"),
            ..Default::default()
        };
        assert!(spa.apply(AXRouter::new()).is_err());

        let spa = Spa {
            must_exist: false,
            ..spa
        };
        assert!(spa.apply(AXRouter::new()).is_ok());
    }

    #[tokio::test]
    async fn serves_files_and_falls_back_to_index() {
        let tree = build_folder();
        let app = router(&spa(&tree.root, "/app")).await;

        let (status, cache, body) = call(&app, Method::GET, "/app/assets/app-abc123.js").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache.as_deref(), Some(IMMUTABLE));
        assert_eq!(body, "console.log(1)");

        for uri in ["/app", "/app/", "/app/index.html", "/app/posts/42"] {
            let (status, cache, body) = call(&app, Method::GET, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert_eq!(cache.as_deref(), Some(NO_CACHE), "{uri}");
            assert_eq!(body, "<h1>shell</h1>", "{uri}");
        }

        let (status, cache, _) = call(&app, Method::HEAD, "/app/posts/42").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache.as_deref(), Some(NO_CACHE));
    }

    #[tokio::test]
    async fn only_files_under_assets_prefix_are_immutable() {
        let tree = build_folder();
        let app = router(&spa(&tree.root, "/app")).await;

        let (status, cache, body) = call(&app, Method::GET, "/app/robots.txt").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache.as_deref(), Some(NO_CACHE));
        assert_eq!(body, "ok");

        let mut spa = spa(&tree.root, "/");
        spa.cache_control.assets_prefix = String::new();
        let app = router(&spa).await;
        let (_, cache, _) = call(&app, Method::GET, "/robots.txt").await;
        assert_eq!(cache.as_deref(), Some(IMMUTABLE));
    }

    #[tokio::test]
    async fn not_modified_keeps_the_shell_revalidated() {
        let tree = build_folder();
        for (mount, prefix) in [("/app", "/app"), ("/", "")] {
            let app = router(&spa(&tree.root, mount)).await;
            let cases = [
                ("/", NO_CACHE),
                ("/index.html", NO_CACHE),
                ("/posts/42", NO_CACHE),
                ("/robots.txt", NO_CACHE),
                ("/assets/app-abc123.js", IMMUTABLE),
            ];
            for (path, expected) in cases {
                let uri = format!("{prefix}{path}");
                let (status, cache, _) = send(
                    &app,
                    Request::builder()
                        .uri(&uri)
                        .header(IF_MODIFIED_SINCE, "Fri, 01 Jan 2100 00:00:00 GMT"),
                )
                .await;
                assert_eq!(status, StatusCode::NOT_MODIFIED, "{uri}");
                assert_eq!(cache.as_deref(), Some(expected), "{uri}");
            }
        }
    }

    #[tokio::test]
    async fn directories_get_the_shell_not_a_redirect() {
        let tree = build_folder();
        let app = router(&spa(&tree.root, "/app")).await;

        for uri in ["/app/docs", "/app/docs/"] {
            let res = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK, "{uri}");
            assert!(res.headers().get(LOCATION).is_none(), "{uri}");
            let body = axum::body::to_bytes(res.into_body(), usize::MAX)
                .await
                .unwrap();
            assert_eq!(body, "<h1>shell</h1>", "{uri}");
        }

        let (status, _, body) = call(&app, Method::GET, "/app/docs/index.html").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "<h1>docs</h1>");
    }

    #[tokio::test]
    async fn routes_under_the_mount_win() {
        let tree = build_folder();
        let app = spa(&tree.root, "/app")
            .apply(AXRouter::new().route("/app/api", get(|| async { "api" })))
            .expect("apply spa")
            .with_state(crate::tests_cfg::app::get_app_context().await);

        let (_, _, body) = call(&app, Method::GET, "/app/api").await;
        assert_eq!(body, "api");
        let (_, _, body) = call(&app, Method::GET, "/app/posts").await;
        assert_eq!(body, "<h1>shell</h1>");
    }

    #[tokio::test]
    async fn missing_files_and_other_methods_are_not_found() {
        let tree = build_folder();
        let app = router(&spa(&tree.root, "/app")).await;

        let (status, cache, body) = call(&app, Method::GET, "/app/assets/old-999.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(cache, None);
        assert_ne!(body, "<h1>shell</h1>");

        let (status, _, _) = call(&app, Method::POST, "/app/posts/42").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn mounted_at_root_serves_unmatched_paths() {
        let tree = build_folder();
        let app = router(&spa(&tree.root, "/")).await;

        let (status, _, body) = call(&app, Method::GET, "/robots.txt").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");

        let (status, cache, body) = call(&app, Method::GET, "/settings/profile").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache.as_deref(), Some(NO_CACHE));
        assert_eq!(body, "<h1>shell</h1>");
    }

    #[tokio::test]
    async fn custom_and_disabled_cache_control() {
        let tree = build_folder();
        let mut spa = spa(&tree.root, "/app");
        spa.cache_control = CacheControl {
            assets: Some("max-age=60".to_string()),
            assets_prefix: "/assets/".to_string(),
            html: None,
            other: Some("max-age=300".to_string()),
        };
        let app = router(&spa).await;

        let (_, cache, _) = call(&app, Method::GET, "/app/assets/app-abc123.js").await;
        assert_eq!(cache.as_deref(), Some("max-age=60"));
        let (_, cache, _) = call(&app, Method::GET, "/app/robots.txt").await;
        assert_eq!(cache.as_deref(), Some("max-age=300"));
        let (_, cache, _) = call(&app, Method::GET, "/app/posts").await;
        assert_eq!(cache, None);
    }

    #[tokio::test]
    async fn serves_precompressed_files() {
        let tree = tree_fs::TreeBuilder::default()
            .drop(true)
            .add("dist/index.html", "<h1>shell</h1>")
            .add("dist/app.js", "plain")
            .add("dist/app.js.gz", "gzipped")
            .create()
            .expect("create build folder");
        let mut spa = spa(&tree.root, "/");
        spa.precompressed = true;
        let app = router(&spa).await;

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/app.js")
                    .header(ACCEPT_ENCODING, "gzip")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.headers().get("content-encoding").unwrap(), "gzip");
    }
}
