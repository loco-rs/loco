//! Forgery protection: refuses state-changing requests a browser sent on
//! behalf of another site (cross-site request forgery). Rails'
//! `protect_from_forgery`.
//!
//! Two modes, set with `mode:`:
//!
//! * `header` (the default) checks where the request came from. Browsers say
//!   so in `Sec-Fetch-Site`, and older ones in `Origin`. A cross-site
//!   `POST`/`PUT`/`PATCH`/`DELETE` is refused with `403`. A request with
//!   neither header did not come from a browser page (curl, a script, a
//!   server-to-server call) and passes. Nothing to add to forms.
//! * `token` is the classic Rails scheme: every unsafe request must carry the
//!   authenticity token, either in the `X-CSRF-Token` header or in the
//!   `authenticity_token` form field. Templates get it from the
//!   `csrf_field()`, `csrf_meta_tags()` and `csrf_token()` Tera functions,
//!   handlers from the [`CsrfToken`] extractor. Needs the `auth_session`
//!   feature: the token lives in a signed cookie keyed off
//!   `auth.session.secret`.
//!
//! Paths under an `exempt` prefix (webhooks, a bearer-token API) are never
//! checked.
//!
//! ```yaml
//! server:
//!   middlewares:
//!     forgery_protection:
//!       enable: true
//!       mode: header
//!       exempt:
//!         - /api/webhooks/
//! ```

use axum::{
    extract::Request,
    http::{header, HeaderMap, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Router as AXRouter,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppContext,
    controller::{middleware::MiddlewareLayer, ErrorDetail},
    Error, Result,
};

/// Default name of the form field that carries the token.
pub const DEFAULT_FIELD: &str = "authenticity_token";
/// Default name of the request header that carries the token.
pub const DEFAULT_HEADER: &str = "x-csrf-token";
/// Default name of the cookie that holds the token.
pub const DEFAULT_COOKIE: &str = "loco_csrf";

/// How requests are checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Refuse requests whose `Sec-Fetch-Site` / `Origin` say they came from
    /// another site.
    Header,
    /// Require the authenticity token on every unsafe request.
    Token,
}

/// `server.middlewares.forgery_protection` configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForgeryProtection {
    /// Off unless set.
    #[serde(default)]
    pub enable: bool,
    /// `header` (default) or `token`.
    #[serde(default = "default_mode")]
    pub mode: Mode,
    /// Path prefixes that are never checked, e.g. `/api/webhooks/`.
    #[serde(default)]
    pub exempt: Vec<String>,
    /// Other origins allowed to send unsafe requests in `header` mode, as
    /// `scheme://host[:port]`, e.g. `https://admin.example.com`.
    #[serde(default)]
    pub trusted_origins: Vec<String>,
    /// Form field that carries the token (`token` mode).
    #[serde(default = "default_field")]
    pub field: String,
    /// Request header that carries the token (`token` mode).
    #[serde(default = "default_header")]
    pub header: String,
    /// Cookie that holds the token (`token` mode).
    #[serde(default = "default_cookie")]
    pub cookie: String,
}

impl Default for ForgeryProtection {
    fn default() -> Self {
        super::defaults()
    }
}

const fn default_mode() -> Mode {
    Mode::Header
}

fn default_field() -> String {
    DEFAULT_FIELD.to_string()
}

fn default_header() -> String {
    DEFAULT_HEADER.to_string()
}

fn default_cookie() -> String {
    DEFAULT_COOKIE.to_string()
}

impl ForgeryProtection {
    fn is_exempt(&self, path: &str) -> bool {
        self.exempt.iter().any(|p| path.starts_with(p.as_str()))
    }
}

/// Whether `method` can change state. `GET`, `HEAD`, `OPTIONS` and `TRACE`
/// must not, so they are never checked.
#[must_use]
pub fn is_unsafe(method: &Method) -> bool {
    !matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

/// Whether `headers` say a browser sent the request from another site.
///
/// `Sec-Fetch-Site: same-origin` and `none` (typed in the address bar, a
/// bookmark) pass. Without `Sec-Fetch-Site`, `Origin` must match `Host`.
/// Without either header the request is not from a browser page and passes.
/// An `Origin` listed in `trusted` always passes.
#[must_use]
pub fn is_cross_site(headers: &HeaderMap, trusted: &[String]) -> bool {
    let text = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let origin = text(header::ORIGIN.as_str());
    if origin.is_some_and(|o| trusted.iter().any(|t| t.trim_end_matches('/') == o)) {
        return false;
    }
    if let Some(site) = text("sec-fetch-site") {
        return !matches!(site, "same-origin" | "none");
    }
    origin.is_some_and(|origin| {
        let host = origin
            .strip_prefix("https://")
            .or_else(|| origin.strip_prefix("http://"));
        host.is_none() || host != text(header::HOST.as_str())
    })
}

fn refuse(error: &str, description: &str) -> Response {
    Error::CustomError(StatusCode::FORBIDDEN, ErrorDetail::new(error, description)).into_response()
}

/// The configured middleware, ready to apply. Built by
/// [`default_middleware_stack`](super::default_middleware_stack) from the
/// config and the app context (which holds the cookie key).
#[derive(Debug, Clone)]
pub struct Middleware {
    config: ForgeryProtection,
    #[cfg(feature = "auth_session")]
    token: std::result::Result<token::Settings, String>,
}

impl Middleware {
    /// The middleware for `config` in `ctx`.
    #[must_use]
    pub fn new(config: ForgeryProtection, ctx: &AppContext) -> Self {
        #[cfg(feature = "auth_session")]
        let token = if config.enable && config.mode == Mode::Token {
            token::Settings::new(&config, ctx).map_err(|e| e.to_string())
        } else {
            Err("not in token mode".to_string())
        };
        #[cfg(not(feature = "auth_session"))]
        let _ = ctx;
        Self {
            config,
            #[cfg(feature = "auth_session")]
            token,
        }
    }
}

impl MiddlewareLayer for Middleware {
    fn name(&self) -> &'static str {
        "forgery_protection"
    }

    fn is_enabled(&self) -> bool {
        self.config.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(&self.config)
    }

    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        match self.config.mode {
            Mode::Header => {
                let config = std::sync::Arc::new(self.config.clone());
                Ok(app.layer(axum::middleware::from_fn(
                    move |req: Request, next: Next| {
                        let config = config.clone();
                        async move { check_header(&config, req, next).await }
                    },
                )))
            }
            #[cfg(feature = "auth_session")]
            Mode::Token => {
                let settings = std::sync::Arc::new(
                    self.token
                        .clone()
                        .map_err(|e| Error::Message(format!("forgery_protection: {e}")))?,
                );
                Ok(app.layer(axum::middleware::from_fn(
                    move |req: Request, next: Next| {
                        let settings = settings.clone();
                        async move { token::check(&settings, req, next).await }
                    },
                )))
            }
            #[cfg(not(feature = "auth_session"))]
            Mode::Token => Err(Error::Message(
                "forgery_protection: `mode: token` needs the `auth_session` feature".to_string(),
            )),
        }
    }
}

async fn check_header(config: &ForgeryProtection, req: Request, next: Next) -> Response {
    if is_unsafe(req.method())
        && !config.is_exempt(req.uri().path())
        && is_cross_site(req.headers(), &config.trusted_origins)
    {
        tracing::warn!(
            method = %req.method(),
            path = req.uri().path(),
            "forgery_protection: refused a cross-site request"
        );
        return refuse(
            "cross_site_request",
            "this request was sent from another site",
        );
    }
    next.run(req).await
}

#[cfg(feature = "auth_session")]
pub use token::{current_token, CsrfToken};

#[cfg(feature = "auth_session")]
mod token {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use axum::{
        body::{Body, Bytes},
        extract::{FromRequest, FromRequestParts, Request},
        http::{header, request::Parts},
        middleware::Next,
        response::{IntoResponse, Response},
    };
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use rand::RngCore;

    use super::{is_unsafe, refuse, ForgeryProtection};
    use crate::{
        app::AppContext,
        auth::cookies::{self, CookieOptions, Key, SignedCookies},
        controller::middleware::limit_payload::DefaultBodyLimitKind,
        Error, Result,
    };

    const TOKEN_LEN: usize = 32;

    #[derive(Debug, Clone)]
    pub struct Settings {
        config: ForgeryProtection,
        key: Key,
        options: CookieOptions,
        body_limit: usize,
    }

    impl Settings {
        pub fn new(config: &ForgeryProtection, ctx: &AppContext) -> Result<Self> {
            let body_limit = match ctx
                .config
                .server
                .middlewares
                .limit_payload
                .clone()
                .unwrap_or_default()
                .body_limit
            {
                DefaultBodyLimitKind::Disable => usize::MAX,
                DefaultBodyLimitKind::Limit(limit) => limit,
            };
            Ok(Self {
                config: config.clone(),
                key: cookies::key(ctx)?,
                options: CookieOptions::from_config(&ctx.config),
                body_limit,
            })
        }
    }

    #[derive(Debug)]
    struct Inner {
        raw: [u8; TOKEN_LEN],
        field: String,
        header: String,
        /// Set once a page used the token, so the cookie is only sent then.
        used: AtomicBool,
    }

    /// The request's authenticity token (`mode: token`).
    ///
    /// Put [`masked`](Self::masked) in a form's hidden field (named
    /// [`field`](Self::field)) or send it in the [`header`](Self::header).
    /// Each call to `masked` returns a different string for the same token,
    /// so the token does not leak through compressed responses (BREACH).
    ///
    /// Templates rendered during the request reach the same token through the
    /// `csrf_token()`, `csrf_field()` and `csrf_meta_tags()` Tera functions.
    #[derive(Debug, Clone)]
    pub struct CsrfToken(Arc<Inner>);

    tokio::task_local! {
        static CURRENT: CsrfToken;
    }

    /// The token of the request being handled, if `forgery_protection` runs
    /// in `token` mode. Used by the Tera functions.
    #[must_use]
    pub fn current_token() -> Option<CsrfToken> {
        CURRENT.try_with(Clone::clone).ok()
    }

    impl CsrfToken {
        pub(super) fn new(raw: [u8; TOKEN_LEN], config: &ForgeryProtection) -> Self {
            Self(Arc::new(Inner {
                raw,
                field: config.field.clone(),
                header: config.header.clone(),
                used: AtomicBool::new(false),
            }))
        }

        /// A masked copy of the token, safe to embed in a page.
        #[must_use]
        pub fn masked(&self) -> String {
            self.0.used.store(true, Ordering::Relaxed);
            let mut out = [0u8; TOKEN_LEN * 2];
            rand::rng().fill_bytes(&mut out[..TOKEN_LEN]);
            for i in 0..TOKEN_LEN {
                out[TOKEN_LEN + i] = out[i] ^ self.0.raw[i];
            }
            URL_SAFE_NO_PAD.encode(out)
        }

        /// The form field name the middleware reads.
        #[must_use]
        pub fn field(&self) -> &str {
            &self.0.field
        }

        /// The request header the middleware reads.
        #[must_use]
        pub fn header(&self) -> &str {
            &self.0.header
        }

        /// Whether `candidate` (a value from [`masked`](Self::masked)) matches
        /// this token. Constant-time.
        #[must_use]
        pub fn verify(&self, candidate: &str) -> bool {
            let Ok(bytes) = URL_SAFE_NO_PAD.decode(candidate.trim()) else {
                return false;
            };
            if bytes.len() != TOKEN_LEN * 2 {
                return false;
            }
            let mut diff = 0u8;
            for i in 0..TOKEN_LEN {
                diff |= (bytes[i] ^ bytes[TOKEN_LEN + i]) ^ self.0.raw[i];
            }
            diff == 0
        }
    }

    impl<S: Send + Sync> FromRequestParts<S> for CsrfToken {
        type Rejection = Error;

        #[allow(clippy::unused_async_trait_impl)]
        async fn from_request_parts(
            parts: &mut Parts,
            _state: &S,
        ) -> std::result::Result<Self, Self::Rejection> {
            parts.extensions.get::<Self>().cloned().ok_or_else(|| {
                Error::Message(
                    "CsrfToken needs `server.middlewares.forgery_protection` with `mode: token`"
                        .to_string(),
                )
            })
        }
    }

    fn decode_raw(value: &str) -> Option<[u8; TOKEN_LEN]> {
        URL_SAFE_NO_PAD.decode(value).ok()?.try_into().ok()
    }

    fn content_type(req: &Request) -> &str {
        req.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
    }

    /// The token the request carries: the header, else the form field.
    /// Returns the request rebuilt around its (possibly buffered) body.
    async fn submitted(
        settings: &Settings,
        req: Request,
    ) -> std::result::Result<(Option<String>, Request), Box<Response>> {
        if let Some(value) = req
            .headers()
            .get(settings.config.header.as_str())
            .and_then(|v| v.to_str().ok())
        {
            return Ok((Some(value.to_string()), req));
        }
        let ctype = content_type(&req).to_ascii_lowercase();
        let is_form = ctype.starts_with("application/x-www-form-urlencoded");
        let is_multipart = ctype.starts_with("multipart/form-data");
        if !is_form && !is_multipart {
            return Ok((None, req));
        }

        let (parts, body) = req.into_parts();
        let bytes = axum::body::to_bytes(body, settings.body_limit)
            .await
            .map_err(|_| {
                Box::new(
                    Error::CustomError(
                        axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                        crate::controller::ErrorDetail::new(
                            "payload_too_large",
                            "request body is too large",
                        ),
                    )
                    .into_response(),
                )
            })?;
        let field = settings.config.field.as_str();
        let found = if is_form {
            url::form_urlencoded::parse(&bytes)
                .find(|(k, _)| k == field)
                .map(|(_, v)| v.into_owned())
        } else {
            multipart_field(&parts, bytes.clone(), field).await
        };
        Ok((found, Request::from_parts(parts, Body::from(bytes))))
    }

    async fn multipart_field(parts: &Parts, bytes: Bytes, field: &str) -> Option<String> {
        let mut req = Request::new(Body::from(bytes));
        if let Some(ctype) = parts.headers.get(header::CONTENT_TYPE) {
            req.headers_mut()
                .insert(header::CONTENT_TYPE, ctype.clone());
        }
        let mut multipart = axum::extract::Multipart::from_request(req, &())
            .await
            .ok()?;
        while let Ok(Some(part)) = multipart.next_field().await {
            if part.name() == Some(field) {
                return part.text().await.ok();
            }
        }
        None
    }

    pub async fn check(settings: &Settings, req: Request, next: Next) -> Response {
        let jar = SignedCookies::from_headers(
            req.headers(),
            settings.key.clone(),
            settings.options.clone(),
        );
        let existing = jar
            .get(&settings.config.cookie)
            .and_then(|c| decode_raw(c.value()));
        let raw = existing.unwrap_or_else(|| {
            let mut raw = [0u8; TOKEN_LEN];
            rand::rng().fill_bytes(&mut raw);
            raw
        });
        let token = CsrfToken::new(raw, &settings.config);

        let mut req = req;
        if is_unsafe(req.method()) && !settings.config.is_exempt(req.uri().path()) {
            let (candidate, rebuilt) = match submitted(settings, req).await {
                Ok(found) => found,
                Err(res) => return *res,
            };
            req = rebuilt;
            let valid = existing.is_some() && candidate.is_some_and(|c| token.verify(&c));
            if !valid {
                tracing::warn!(
                    method = %req.method(),
                    path = req.uri().path(),
                    "forgery_protection: missing or invalid authenticity token"
                );
                return refuse(
                    "invalid_authenticity_token",
                    "the authenticity token is missing or invalid",
                );
            }
        }

        req.extensions_mut().insert(token.clone());
        let res = CURRENT.scope(token.clone(), next.run(req)).await;

        if existing.is_none() && token.0.used.load(Ordering::Relaxed) {
            let cookie = settings
                .options
                .cookie(settings.config.cookie.clone(), URL_SAFE_NO_PAD.encode(raw));
            return (jar.add(cookie), res).into_response();
        }
        res
    }
}

/// Tera functions for `token` mode: `csrf_token()`, `csrf_field()` and
/// `csrf_meta_tags()`. Registered on every Loco Tera instance; they fail to
/// render outside a request protected in `token` mode.
#[cfg(feature = "auth_session")]
pub(crate) mod tera_functions {
    use tera::{Kwargs, State, TeraResult, Value};

    use super::{current_token, CsrfToken};

    fn token() -> TeraResult<CsrfToken> {
        current_token().ok_or_else(|| {
            tera::Error::message(
                "csrf functions need `server.middlewares.forgery_protection` with `mode: token`",
            )
        })
    }

    fn escape(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    #[allow(clippy::needless_pass_by_value)]
    fn csrf_token(_: Kwargs, _: &State<'_>) -> TeraResult<Value> {
        Ok(Value::from(token()?.masked()))
    }

    #[allow(clippy::needless_pass_by_value)]
    fn csrf_field(_: Kwargs, _: &State<'_>) -> TeraResult<Value> {
        let token = token()?;
        Ok(Value::safe_string(&format!(
            r#"<input type="hidden" name="{}" value="{}">"#,
            escape(token.field()),
            token.masked()
        )))
    }

    #[allow(clippy::needless_pass_by_value)]
    fn csrf_meta_tags(_: Kwargs, _: &State<'_>) -> TeraResult<Value> {
        let token = token()?;
        Ok(Value::safe_string(&format!(
            r#"<meta name="csrf-param" content="{}"><meta name="csrf-token" content="{}">"#,
            escape(token.field()),
            token.masked()
        )))
    }

    pub fn register(tera: &mut tera::Tera) {
        tera.register_function("csrf_token", csrf_token);
        tera.register_function("csrf_field", csrf_field);
        tera.register_function("csrf_meta_tags", csrf_meta_tags);
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        map
    }

    #[test]
    fn defaults() {
        let config = ForgeryProtection::default();
        assert!(!config.enable);
        assert_eq!(config.mode, Mode::Header);
        assert_eq!(config.field, DEFAULT_FIELD);
        assert_eq!(config.header, DEFAULT_HEADER);
        assert_eq!(config.cookie, DEFAULT_COOKIE);
        assert!(config.exempt.is_empty());
    }

    #[test]
    fn only_unsafe_methods_are_checked() {
        assert!(!is_unsafe(&Method::GET));
        assert!(!is_unsafe(&Method::HEAD));
        assert!(!is_unsafe(&Method::OPTIONS));
        assert!(is_unsafe(&Method::POST));
        assert!(is_unsafe(&Method::PUT));
        assert!(is_unsafe(&Method::PATCH));
        assert!(is_unsafe(&Method::DELETE));
    }

    #[test]
    fn cross_site_detection() {
        let cross = |h: &[(&str, &str)]| is_cross_site(&headers(h), &[]);
        assert!(!cross(&[]));
        assert!(!cross(&[("sec-fetch-site", "same-origin")]));
        assert!(!cross(&[("sec-fetch-site", "none")]));
        assert!(cross(&[("sec-fetch-site", "cross-site")]));
        assert!(cross(&[("sec-fetch-site", "same-site")]));
        assert!(!cross(&[
            ("origin", "https://app.test"),
            ("host", "app.test")
        ]));
        assert!(!cross(&[
            ("origin", "http://localhost:5150"),
            ("host", "localhost:5150")
        ]));
        assert!(cross(&[
            ("origin", "https://evil.test"),
            ("host", "app.test")
        ]));
        assert!(cross(&[("origin", "null"), ("host", "app.test")]));
    }

    #[test]
    fn trusted_origins_pass() {
        let trusted = vec!["https://admin.test/".to_string()];
        assert!(!is_cross_site(
            &headers(&[
                ("sec-fetch-site", "same-site"),
                ("origin", "https://admin.test")
            ]),
            &trusted
        ));
        assert!(is_cross_site(
            &headers(&[
                ("sec-fetch-site", "cross-site"),
                ("origin", "https://evil.test")
            ]),
            &trusted
        ));
    }

    #[test]
    fn exempt_prefixes() {
        let config = ForgeryProtection {
            exempt: vec!["/api/webhooks/".to_string()],
            ..Default::default()
        };
        assert!(config.is_exempt("/api/webhooks/github"));
        assert!(!config.is_exempt("/api/posts"));
    }

    #[test]
    fn mode_from_yaml() {
        let config: ForgeryProtection =
            serde_yaml::from_str("enable: true\nmode: token\nexempt: [/hooks/]").unwrap();
        assert!(config.enable);
        assert_eq!(config.mode, Mode::Token);
        assert_eq!(config.exempt, vec!["/hooks/".to_string()]);
    }

    #[cfg(feature = "auth_session")]
    #[test]
    fn masked_tokens_differ_and_verify() {
        let token = token::CsrfToken::new([7u8; 32], &ForgeryProtection::default());
        let a = token.masked();
        let b = token.masked();
        assert_ne!(a, b);
        assert!(token.verify(&a));
        assert!(token.verify(&b));
        assert!(!token.verify("nope"));
        let other = token::CsrfToken::new([8u8; 32], &ForgeryProtection::default());
        assert!(!other.verify(&a));
    }
}
