//! Signed (and, with `auth_session_encrypted`, encrypted) cookies keyed off
//! `auth.session.secret` — Rails' `cookies.signed` / `cookies.encrypted`.
//!
//! A **signed** cookie is readable by the browser but tamper-evident: any
//! change to its value, or a value signed with another key, reads back as
//! absent. An **encrypted** cookie is also confidential.
//!
//! ```rust
//! use loco_rs::prelude::*;
//! use loco_rs::auth::cookies::SignedCookies;
//!
//! async fn remember_theme(cookies: SignedCookies) -> Result<impl IntoResponse> {
//!     let cookie = cookies.options().cookie("theme", "dark");
//!     Ok((cookies.add(cookie), "saved"))
//! }
//!
//! async fn theme(cookies: SignedCookies) -> Result<Response> {
//!     let theme = cookies.get("theme").map(|c| c.value().to_string());
//!     format::json(theme)
//! }
//! ```
//!
//! Both extractors fail with `500` in production when `auth.session.secret`
//! is unset (the development default would let anyone forge cookies), and
//! `cargo loco doctor --production` reports it.
//!
//! [`seal`]/[`unseal`] put an expiry *inside* the signed value, so a cookie
//! stops being accepted at a server-chosen time no matter what the browser
//! does with `Max-Age`. The session and the OAuth state cookie use them.

use std::sync::OnceLock;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    response::{IntoResponseParts, ResponseParts},
};
#[cfg(feature = "auth_session_encrypted")]
pub use axum_extra::extract::cookie::PrivateCookieJar;
pub use axum_extra::extract::cookie::{Cookie, Key, SameSite, SignedCookieJar};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha512};

use crate::{
    app::AppContext,
    config::{Config, SessionConfig, DEV_SESSION_SECRET},
    environment::Environment,
    Error, Result,
};

/// Default session cookie name.
pub const DEFAULT_SESSION_COOKIE: &str = "loco_session";
/// Default session lifetime: 30 days.
pub const DEFAULT_SESSION_MAX_AGE: u64 = 60 * 60 * 24 * 30;

/// Derives the cookie [`Key`] from a secret of any length.
///
/// `cookie::Key` wants 64 bytes (32 for signing, 32 for encryption).
/// SHA-512 over a domain-separated secret gives exactly that, so a short
/// human-chosen secret still yields a full-width key — it is only as strong
/// as the secret, which is what the doctor check is for.
#[must_use]
pub fn derive_key(secret: &str) -> Key {
    let mut hasher = Sha512::new();
    hasher.update(b"loco-rs cookie key v1\0");
    hasher.update(secret.as_bytes());
    Key::from(&hasher.finalize())
}

/// Whether `config` leaves the session secret at its development default.
#[must_use]
pub fn uses_default_secret(config: &Config) -> bool {
    session_config(config)
        .and_then(|s| s.secret.as_deref())
        .is_none_or(|s| s.is_empty() || s == DEV_SESSION_SECRET)
}

fn session_config(config: &Config) -> Option<&SessionConfig> {
    config.auth.as_ref().and_then(|a| a.session.as_ref())
}

/// The cookie key for this app.
///
/// # Errors
///
/// In production, when `auth.session.secret` is unset or left at the
/// development default.
pub fn key(ctx: &AppContext) -> Result<Key> {
    key_for(&ctx.config, &ctx.environment)
}

/// [`key`] from a config and environment, for code without an `AppContext`.
///
/// # Errors
///
/// In production, when `auth.session.secret` is unset or left at the
/// development default.
pub fn key_for(config: &Config, environment: &Environment) -> Result<Key> {
    static WARNED: OnceLock<()> = OnceLock::new();
    if uses_default_secret(config) {
        if *environment == Environment::Production {
            tracing::error!(
                "auth.session.secret is not set; refusing to sign cookies with the development \
                 default in production"
            );
            return Err(Error::Message(
                "auth.session.secret is not set. Signed cookies would be forgeable with the \
                 development default; set it (e.g. `openssl rand -base64 64`) for production."
                    .to_string(),
            ));
        }
        WARNED.get_or_init(|| {
            tracing::warn!(
                "auth.session.secret is not set; signing cookies with the development default"
            );
        });
        return Ok(derive_key(DEV_SESSION_SECRET));
    }
    let secret = session_config(config)
        .and_then(|s| s.secret.as_deref())
        .unwrap_or(DEV_SESSION_SECRET);
    Ok(derive_key(secret))
}

/// Cookie attributes Loco applies to the cookies it sets: `Path=/`,
/// `HttpOnly`, `SameSite=Lax`, `Secure` on https, and a `Max-Age`.
///
/// `SameSite=Lax` (not `Strict`) is deliberate: the OAuth callback is a
/// top-level navigation *from the provider's site*, and a `Strict` cookie is
/// not sent on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieOptions {
    /// Set the `Secure` flag.
    pub secure: bool,
    /// `Max-Age` in seconds; `None` for a browser-session cookie.
    pub max_age: Option<u64>,
}

impl CookieOptions {
    /// The options `config` asks for, with `max_age` from
    /// `auth.session.max_age`.
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        let session = session_config(config);
        let secure = session
            .and_then(|s| s.secure)
            .unwrap_or_else(|| config.server.host.starts_with("https://"));
        Self {
            secure,
            max_age: Some(
                session
                    .and_then(|s| s.max_age)
                    .unwrap_or(DEFAULT_SESSION_MAX_AGE),
            ),
        }
    }

    /// Same options with another `Max-Age`.
    #[must_use]
    pub const fn with_max_age(mut self, max_age: Option<u64>) -> Self {
        self.max_age = max_age;
        self
    }

    /// A cookie carrying these attributes.
    #[must_use]
    pub fn cookie(&self, name: impl Into<String>, value: impl Into<String>) -> Cookie<'static> {
        let mut builder = Cookie::build((name.into(), value.into()))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.secure);
        if let Some(max_age) = self.max_age {
            let seconds = i64::try_from(max_age).unwrap_or(i64::MAX);
            builder = builder.max_age(cookie::time::Duration::seconds(seconds));
        }
        builder.build()
    }

    /// A cookie that deletes `name` (empty value, `Max-Age=0`), with the same
    /// path so the browser matches it.
    #[must_use]
    pub fn removal(&self, name: impl Into<String>) -> Cookie<'static> {
        let mut cookie = self.cookie(name, "");
        cookie.make_removal();
        cookie
    }
}

#[derive(Serialize, Deserialize)]
struct Sealed<T> {
    /// Unix seconds after which the value is rejected.
    exp: i64,
    v: T,
}

/// Encodes `value` with an expiry `ttl_seconds` from now, as cookie-safe
/// text (base64url JSON). Put the result in a signed or encrypted cookie;
/// [`unseal`] reads it back.
///
/// # Errors
///
/// When `value` does not serialize.
pub fn seal<T: Serialize>(value: &T, ttl_seconds: u64) -> Result<String> {
    let ttl = i64::try_from(ttl_seconds).unwrap_or(i64::MAX);
    let exp = chrono::Utc::now().timestamp().saturating_add(ttl);
    let json = serde_json::to_vec(&Sealed { exp, v: value })?;
    Ok(URL_SAFE_NO_PAD.encode(json))
}

/// Reads a value written by [`seal`], or `None` when it is malformed or
/// expired.
#[must_use]
pub fn unseal<T: DeserializeOwned>(text: &str) -> Option<T> {
    unseal_at(text, chrono::Utc::now().timestamp())
}

fn unseal_at<T: DeserializeOwned>(text: &str, now: i64) -> Option<T> {
    let bytes = URL_SAFE_NO_PAD.decode(text).ok()?;
    let sealed: Sealed<T> = serde_json::from_slice(&bytes).ok()?;
    (sealed.exp > now).then_some(sealed.v)
}

/// A signed cookie jar keyed off `auth.session.secret`.
///
/// Wraps axum-extra's [`SignedCookieJar`]; [`get`](Self::get) returns only
/// cookies whose signature verifies. Return the jar from the handler (it is
/// [`IntoResponseParts`]) for additions and removals to reach the browser.
#[derive(Debug, Clone)]
pub struct SignedCookies {
    jar: SignedCookieJar,
    options: CookieOptions,
}

impl SignedCookies {
    /// A jar over `headers`' cookies, verified with `key`.
    #[must_use]
    pub fn from_headers(headers: &axum::http::HeaderMap, key: Key, options: CookieOptions) -> Self {
        Self {
            jar: SignedCookieJar::from_headers(headers, key),
            options,
        }
    }

    /// The verified cookie `name`, if present and untampered.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.jar.get(name)
    }

    /// Signs and adds `cookie`.
    #[must_use]
    #[allow(clippy::should_implement_trait)] // mirrors axum-extra's jar API
    pub fn add(mut self, cookie: impl Into<Cookie<'static>>) -> Self {
        self.jar = self.jar.add(cookie);
        self
    }

    /// Removes the cookie `name`, using the app's cookie attributes so the
    /// browser matches it.
    #[must_use]
    pub fn remove(mut self, name: &str) -> Self {
        self.jar = self.jar.remove(self.options.removal(name.to_string()));
        self
    }

    /// The app's cookie attributes, for building cookies to [`add`](Self::add).
    #[must_use]
    pub const fn options(&self) -> &CookieOptions {
        &self.options
    }

    /// The underlying axum-extra jar.
    pub fn into_inner(self) -> SignedCookieJar {
        self.jar
    }
}

impl<S> FromRequestParts<S> for SignedCookies
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    // Header and config inspection only; the shape is axum's.
    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        Ok(Self::from_headers(
            &parts.headers,
            key(&ctx)?,
            CookieOptions::from_config(&ctx.config),
        ))
    }
}

impl IntoResponseParts for SignedCookies {
    type Error = std::convert::Infallible;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        self.jar.into_response_parts(res)
    }
}

/// An encrypted (AES-256-GCM) cookie jar keyed off `auth.session.secret`:
/// the browser can neither read nor alter the values.
#[cfg(feature = "auth_session_encrypted")]
#[derive(Debug, Clone)]
pub struct EncryptedCookies {
    jar: PrivateCookieJar,
    options: CookieOptions,
}

#[cfg(feature = "auth_session_encrypted")]
impl EncryptedCookies {
    /// A jar over `headers`' cookies, decrypted with `key`.
    #[must_use]
    pub fn from_headers(headers: &axum::http::HeaderMap, key: Key, options: CookieOptions) -> Self {
        Self {
            jar: PrivateCookieJar::from_headers(headers, key),
            options,
        }
    }

    /// The decrypted cookie `name`, if present and authentic.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.jar.get(name)
    }

    /// Encrypts and adds `cookie`.
    #[must_use]
    #[allow(clippy::should_implement_trait)] // mirrors axum-extra's jar API
    pub fn add(mut self, cookie: impl Into<Cookie<'static>>) -> Self {
        self.jar = self.jar.add(cookie);
        self
    }

    /// Removes the cookie `name`.
    #[must_use]
    pub fn remove(mut self, name: &str) -> Self {
        self.jar = self.jar.remove(self.options.removal(name.to_string()));
        self
    }

    /// The app's cookie attributes.
    #[must_use]
    pub const fn options(&self) -> &CookieOptions {
        &self.options
    }

    /// The underlying axum-extra jar.
    pub fn into_inner(self) -> PrivateCookieJar {
        self.jar
    }
}

#[cfg(feature = "auth_session_encrypted")]
impl<S> FromRequestParts<S> for EncryptedCookies
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        Ok(Self::from_headers(
            &parts.headers,
            key(&ctx)?,
            CookieOptions::from_config(&ctx.config),
        ))
    }
}

#[cfg(feature = "auth_session_encrypted")]
impl IntoResponseParts for EncryptedCookies {
    type Error = std::convert::Infallible;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        self.jar.into_response_parts(res)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use axum::{
        http::{header, HeaderMap, HeaderValue},
        response::IntoResponse,
    };

    use super::*;

    /// The `name=value` pairs a response sets, as a browser would send them
    /// back.
    pub(crate) fn echo_cookies(res: &axum::response::Response) -> HeaderMap {
        let pairs: Vec<String> = res
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .filter_map(|v| v.split(';').next())
            .map(str::to_string)
            .collect();
        let mut headers = HeaderMap::new();
        if !pairs.is_empty() {
            headers.insert(
                header::COOKIE,
                HeaderValue::from_str(&pairs.join("; ")).unwrap(),
            );
        }
        headers
    }

    fn options() -> CookieOptions {
        CookieOptions {
            secure: false,
            max_age: Some(60),
        }
    }

    fn signed_response(key: &Key, name: &str, value: &str) -> axum::response::Response {
        let jar = SignedCookies::from_headers(&HeaderMap::new(), key.clone(), options());
        let cookie = jar.options().cookie(name.to_string(), value.to_string());
        (jar.add(cookie), "ok").into_response()
    }

    #[test]
    fn a_signed_cookie_round_trips() {
        let key = derive_key("a-secret-of-reasonable-length-for-testing");
        let res = signed_response(&key, "theme", "dark");
        let jar = SignedCookies::from_headers(&echo_cookies(&res), key, options());
        assert_eq!(jar.get("theme").unwrap().value(), "dark");
    }

    #[test]
    fn a_tampered_value_reads_as_absent() {
        let key = derive_key("a-secret-of-reasonable-length-for-testing");
        let res = signed_response(&key, "user", "1");
        let original = echo_cookies(&res)
            .get(header::COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(original.ends_with('1'));
        let forged = format!("{}2", &original[..original.len() - 1]);
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_str(&forged).unwrap());

        let jar = SignedCookies::from_headers(&headers, key, options());
        assert!(jar.get("user").is_none(), "a changed value must not verify");
    }

    #[test]
    fn an_unsigned_value_reads_as_absent() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_static("user=1"));
        let jar = SignedCookies::from_headers(&headers, derive_key("k"), options());
        assert!(jar.get("user").is_none());
    }

    #[test]
    fn a_value_signed_with_another_key_reads_as_absent() {
        let res = signed_response(&derive_key("key-one"), "user", "1");
        let jar =
            SignedCookies::from_headers(&echo_cookies(&res), derive_key("key-two"), options());
        assert!(jar.get("user").is_none());
    }

    #[test]
    fn derived_keys_are_deterministic_and_distinct() {
        assert_eq!(derive_key("same").signing(), derive_key("same").signing());
        assert_ne!(derive_key("one").signing(), derive_key("two").signing());
    }

    #[test]
    fn sealed_values_expire() {
        let text = seal(&"payload", 60).unwrap();
        let now = chrono::Utc::now().timestamp();
        assert_eq!(unseal_at::<String>(&text, now).as_deref(), Some("payload"));
        assert_eq!(unseal_at::<String>(&text, now + 61), None);
        assert_eq!(unseal::<String>("not base64 !!"), None);
        assert_eq!(unseal::<String>(&URL_SAFE_NO_PAD.encode("{}")), None);
    }

    #[test]
    fn cookie_attributes_are_secure_by_default() {
        let cookie = options().cookie("a", "b").to_string();
        assert_eq!(cookie, "a=b; HttpOnly; SameSite=Lax; Path=/; Max-Age=60");

        let secure = CookieOptions {
            secure: true,
            max_age: None,
        };
        assert_eq!(
            secure.cookie("a", "b").to_string(),
            "a=b; HttpOnly; SameSite=Lax; Secure; Path=/"
        );
        assert!(options().removal("a").to_string().contains("Max-Age=0"));
    }

    #[test]
    fn secure_follows_an_https_host_unless_overridden() {
        let mut config = crate::tests_cfg::config::test_config();
        config.server.host = "https://example.com".to_string();
        assert!(CookieOptions::from_config(&config).secure);

        config.server.host = "http://localhost".to_string();
        assert!(!CookieOptions::from_config(&config).secure);

        config.auth = Some(crate::config::Auth {
            session: Some(SessionConfig {
                secure: Some(true),
                max_age: Some(5),
                ..Default::default()
            }),
            ..Default::default()
        });
        let options = CookieOptions::from_config(&config);
        assert!(options.secure);
        assert_eq!(options.max_age, Some(5));
    }

    #[test]
    fn production_refuses_the_development_secret() {
        let mut config = crate::tests_cfg::config::test_config();
        assert!(key_for(&config, &Environment::Development).is_ok());
        assert!(key_for(&config, &Environment::Production).is_err());

        config.auth = Some(crate::config::Auth {
            session: Some(SessionConfig {
                secret: Some(DEV_SESSION_SECRET.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert!(key_for(&config, &Environment::Production).is_err());

        config.auth = Some(crate::config::Auth {
            session: Some(SessionConfig {
                secret: Some("a-real-production-secret-from-the-environment".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert!(key_for(&config, &Environment::Production).is_ok());
    }

    #[cfg(feature = "auth_session_encrypted")]
    #[test]
    fn an_encrypted_cookie_round_trips_and_hides_its_value() {
        let key = derive_key("encryption-secret");
        let jar = EncryptedCookies::from_headers(&HeaderMap::new(), key.clone(), options());
        let cookie = jar.options().cookie("token", "s3cr3t-value");
        let res = (jar.add(cookie), "ok").into_response();
        let sent = echo_cookies(&res);
        assert!(!sent
            .get(header::COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("s3cr3t-value"));

        let jar = EncryptedCookies::from_headers(&sent, key, options());
        assert_eq!(jar.get("token").unwrap().value(), "s3cr3t-value");
        let other = EncryptedCookies::from_headers(&sent, derive_key("other"), options());
        assert!(other.get("token").is_none());
    }
}
