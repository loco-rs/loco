//! Verify signed incoming webhooks before the handler runs.
//!
//! [`SignedWebhook<S>`] is an extractor that reads the raw request body,
//! checks its HMAC-SHA256 signature against the secret configured under
//! `webhooks.<S::NAME>`, and only then hands the handler the verified bytes.
//! The comparison is constant-time. A request that fails is answered `401`
//! (missing, wrong or expired signature) or `400` (malformed signature) and the
//! handler never runs.
//!
//! ```rust
//! use loco_rs::prelude::*;
//! use loco_rs::controller::extractor::webhook::{SignedWebhook, WebhookSource};
//! use serde::Deserialize;
//!
//! /// `webhooks.github` in `config/<env>.yaml`.
//! pub struct GitHub;
//! impl WebhookSource for GitHub {
//!     const NAME: &'static str = "github";
//! }
//!
//! #[derive(Deserialize)]
//! struct Push {
//!     r#ref: String,
//! }
//!
//! async fn github(hook: SignedWebhook<GitHub>) -> Result<Response> {
//!     if hook.header("X-GitHub-Event") == Some("push") {
//!         let push: Push = hook.json()?;
//!         tracing::info!(r#ref = %push.r#ref, "push received");
//!     }
//!     format::empty()
//! }
//!
//! pub fn routes() -> Routes {
//!     Routes::new().prefix("/webhooks").add("/github", post(github))
//! }
//! ```

use std::{fmt::Write as _, marker::PhantomData};

use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde::de::DeserializeOwned;
use sha2::Sha256;

use crate::{
    app::AppContext,
    config::{SignatureEncoding, Webhook, WebhookScheme},
};

type HmacSha256 = Hmac<Sha256>;

/// Default signature header of [`WebhookScheme::Hmac`] (GitHub's).
pub const HMAC_HEADER: &str = "X-Hub-Signature-256";
/// Default signature header of [`WebhookScheme::Timestamped`] (Stripe's).
pub const TIMESTAMPED_HEADER: &str = "Stripe-Signature";
/// Signature header of [`WebhookScheme::Loco`].
pub const LOCO_SIGNATURE_HEADER: &str = "X-Loco-Signature";
/// Timestamp header of [`WebhookScheme::Loco`].
pub const LOCO_TIMESTAMP_HEADER: &str = "X-Loco-Timestamp";
const DEFAULT_PREFIX: &str = "sha256=";

/// Names the `webhooks:` config entry a [`SignedWebhook`] verifies against.
///
/// Implement it on a unit struct per sender:
///
/// ```rust
/// use loco_rs::controller::extractor::webhook::WebhookSource;
///
/// pub struct Stripe;
/// impl WebhookSource for Stripe {
///     const NAME: &'static str = "stripe";
/// }
/// ```
pub trait WebhookSource: Send + Sync + 'static {
    /// Key under `webhooks:` in the app config.
    const NAME: &'static str;
}

/// Why a webhook was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WebhookError {
    /// No `webhooks.<name>` entry in the config.
    #[error("webhook source `{0}` is not configured under `webhooks:`")]
    NotConfigured(String),
    /// The configured secret is empty.
    #[error("webhook secret is empty")]
    EmptySecret,
    /// A header the scheme needs is absent.
    #[error("missing webhook header `{0}`")]
    MissingHeader(String),
    /// The signature header could not be parsed.
    #[error("malformed webhook signature: {0}")]
    Malformed(&'static str),
    /// The signature does not sign this body under the secret.
    #[error("webhook signature does not match")]
    Mismatch,
    /// The signed timestamp is outside the tolerance window.
    #[error("webhook timestamp is outside the tolerance window")]
    Expired,
}

impl From<WebhookError> for crate::Error {
    fn from(err: WebhookError) -> Self {
        match err {
            WebhookError::NotConfigured(_) | WebhookError::EmptySecret => {
                tracing::error!(error = %err, "webhook verification is misconfigured");
                Self::Message(err.to_string())
            }
            WebhookError::Malformed(_) => Self::BadRequest(err.to_string()),
            WebhookError::MissingHeader(_) | WebhookError::Mismatch | WebhookError::Expired => {
                Self::Unauthorized(err.to_string())
            }
        }
    }
}

impl IntoResponse for WebhookError {
    fn into_response(self) -> Response {
        crate::Error::from(self).into_response()
    }
}

/// A request body whose signature has been verified against
/// `webhooks.<S::NAME>`. See the [module docs](self).
///
/// It consumes the body, so it must be the last extractor of the handler.
pub struct SignedWebhook<S: WebhookSource> {
    body: Bytes,
    headers: HeaderMap,
    source: PhantomData<fn() -> S>,
}

impl<S: WebhookSource> std::fmt::Debug for SignedWebhook<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignedWebhook")
            .field("source", &S::NAME)
            .field("body_len", &self.body.len())
            .finish_non_exhaustive()
    }
}

impl<S: WebhookSource> SignedWebhook<S> {
    /// The verified raw body.
    #[must_use]
    pub const fn body(&self) -> &Bytes {
        &self.body
    }

    /// Takes the verified raw body.
    #[must_use]
    pub fn into_body(self) -> Bytes {
        self.body
    }

    /// The request headers (event name, delivery ID, ...).
    #[must_use]
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// One request header as text, if present and valid UTF-8.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Parses the verified body as JSON.
    ///
    /// # Errors
    ///
    /// [`crate::Error::BadRequest`] when the body is not valid JSON for `T`.
    pub fn json<T: DeserializeOwned>(&self) -> crate::Result<T> {
        serde_json::from_slice(&self.body)
            .map_err(|err| crate::Error::BadRequest(format!("invalid webhook JSON: {err}")))
    }
}

impl<S: WebhookSource> FromRequest<AppContext> for SignedWebhook<S> {
    type Rejection = Response;

    async fn from_request(req: Request, ctx: &AppContext) -> Result<Self, Self::Rejection> {
        let Some(config) = ctx.config.webhooks.get(S::NAME) else {
            return Err(WebhookError::NotConfigured(S::NAME.to_string()).into_response());
        };
        let headers = req.headers().clone();
        let body = Bytes::from_request(req, ctx)
            .await
            .map_err(IntoResponse::into_response)?;
        if let Err(err) = verify(config, &headers, &body, chrono::Utc::now().timestamp()) {
            tracing::warn!(source = S::NAME, error = %err, "webhook refused");
            return Err(err.into_response());
        }
        Ok(Self {
            body,
            headers,
            source: PhantomData,
        })
    }
}

/// Checks that `headers` carry a valid signature of `body` under `config`, at
/// time `now` (Unix seconds). The extractor calls this; call it directly to
/// verify outside a handler.
///
/// # Errors
///
/// The [`WebhookError`] describing why the request is refused.
pub fn verify(
    config: &Webhook,
    headers: &HeaderMap,
    body: &[u8],
    now: i64,
) -> Result<(), WebhookError> {
    if config.secret.is_empty() {
        return Err(WebhookError::EmptySecret);
    }
    let secret = config.secret.as_bytes();
    match config.scheme {
        WebhookScheme::Hmac => {
            let value = header(headers, signature_header(config))?;
            let digest = decode(config, strip_prefix(config, value)?)?;
            check(secret, None, body, &digest)
        }
        WebhookScheme::Loco => {
            let value = header(headers, signature_header(config))?;
            let timestamp = header(headers, LOCO_TIMESTAMP_HEADER)?;
            let digest = decode(config, strip_prefix(config, value)?)?;
            check(secret, Some(timestamp), body, &digest)?;
            check_time(config, timestamp, now)
        }
        WebhookScheme::Timestamped => {
            let value = header(headers, signature_header(config))?;
            let mut timestamp = None;
            let mut digests = Vec::new();
            for part in value.split(',') {
                match part.trim().split_once('=') {
                    Some(("t", t)) => timestamp = Some(t),
                    Some(("v1", sig)) => digests.push(decode(config, sig)?),
                    _ => {}
                }
            }
            let timestamp = timestamp.ok_or(WebhookError::Malformed("no `t=` timestamp"))?;
            if digests.is_empty() {
                return Err(WebhookError::Malformed("no `v1=` signature"));
            }
            if !digests
                .iter()
                .any(|digest| check(secret, Some(timestamp), body, digest).is_ok())
            {
                return Err(WebhookError::Mismatch);
            }
            check_time(config, timestamp, now)
        }
    }
}

/// The headers a sender would attach to `body` at `now` (Unix seconds) under
/// `config`: what [`verify`] accepts. Use it in request tests to send a
/// correctly signed webhook.
///
/// ```rust
/// use loco_rs::config::Webhook;
/// use loco_rs::controller::extractor::webhook::{sign, verify};
///
/// let config: Webhook = serde_json::from_value(serde_json::json!({"secret": "s3cret"})).unwrap();
/// let headers = sign(&config, b"{}", 1_700_000_000);
/// assert_eq!(headers[0].0, "X-Hub-Signature-256");
/// ```
#[must_use]
pub fn sign(config: &Webhook, body: &[u8], now: i64) -> Vec<(String, String)> {
    let secret = config.secret.as_bytes();
    let header = signature_header(config).to_string();
    let prefix = config.prefix.as_deref().unwrap_or(DEFAULT_PREFIX);
    match config.scheme {
        WebhookScheme::Hmac => {
            let digest = encode(config, &mac(secret, None, body));
            vec![(header, format!("{prefix}{digest}"))]
        }
        WebhookScheme::Loco => {
            let timestamp = now.to_string();
            let digest = encode(config, &mac(secret, Some(&timestamp), body));
            vec![
                (header, format!("{prefix}{digest}")),
                (LOCO_TIMESTAMP_HEADER.to_string(), timestamp),
            ]
        }
        WebhookScheme::Timestamped => {
            let timestamp = now.to_string();
            let digest = encode(config, &mac(secret, Some(&timestamp), body));
            vec![(header, format!("t={timestamp},v1={digest}"))]
        }
    }
}

fn signature_header(config: &Webhook) -> &str {
    config.header.as_deref().unwrap_or(match config.scheme {
        WebhookScheme::Hmac => HMAC_HEADER,
        WebhookScheme::Timestamped => TIMESTAMPED_HEADER,
        WebhookScheme::Loco => LOCO_SIGNATURE_HEADER,
    })
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, WebhookError> {
    headers
        .get(name)
        .ok_or_else(|| WebhookError::MissingHeader(name.to_string()))?
        .to_str()
        .map(str::trim)
        .map_err(|_| WebhookError::Malformed("header is not text"))
}

fn strip_prefix<'a>(config: &Webhook, value: &'a str) -> Result<&'a str, WebhookError> {
    let prefix = config.prefix.as_deref().unwrap_or(DEFAULT_PREFIX);
    value
        .strip_prefix(prefix)
        .ok_or(WebhookError::Malformed("unexpected signature prefix"))
}

fn decode(config: &Webhook, text: &str) -> Result<Vec<u8>, WebhookError> {
    match config.encoding {
        SignatureEncoding::Hex => {
            if !text.len().is_multiple_of(2) || !text.is_ascii() {
                return Err(WebhookError::Malformed("signature is not hex"));
            }
            (0..text.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&text[i..i + 2], 16))
                .collect::<Result<_, _>>()
                .map_err(|_| WebhookError::Malformed("signature is not hex"))
        }
        SignatureEncoding::Base64 => base64::engine::general_purpose::STANDARD
            .decode(text)
            .map_err(|_| WebhookError::Malformed("signature is not base64")),
    }
}

fn encode(config: &Webhook, digest: &[u8]) -> String {
    match config.encoding {
        SignatureEncoding::Hex => {
            digest
                .iter()
                .fold(String::with_capacity(digest.len() * 2), |mut out, byte| {
                    let _ = write!(out, "{byte:02x}");
                    out
                })
        }
        SignatureEncoding::Base64 => base64::engine::general_purpose::STANDARD.encode(digest),
    }
}

fn hmac(secret: &[u8], timestamp: Option<&str>, body: &[u8]) -> HmacSha256 {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    if let Some(timestamp) = timestamp {
        mac.update(timestamp.as_bytes());
        mac.update(b".");
    }
    mac.update(body);
    mac
}

fn mac(secret: &[u8], timestamp: Option<&str>, body: &[u8]) -> Vec<u8> {
    hmac(secret, timestamp, body)
        .finalize()
        .into_bytes()
        .to_vec()
}

/// Constant-time: `verify_slice` compares the full digest without
/// short-circuiting.
fn check(
    secret: &[u8],
    timestamp: Option<&str>,
    body: &[u8],
    digest: &[u8],
) -> Result<(), WebhookError> {
    hmac(secret, timestamp, body)
        .verify_slice(digest)
        .map_err(|_| WebhookError::Mismatch)
}

fn check_time(config: &Webhook, timestamp: &str, now: i64) -> Result<(), WebhookError> {
    let sent: i64 = timestamp
        .parse()
        .map_err(|_| WebhookError::Malformed("timestamp is not a number"))?;
    if now.abs_diff(sent) > config.tolerance_secs {
        return Err(WebhookError::Expired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    const NOW: i64 = 1_700_000_000;
    const BODY: &[u8] = br#"{"action":"opened"}"#;

    fn config(json: serde_json::Value) -> Webhook {
        serde_json::from_value(json).expect("config")
    }

    fn headers(pairs: &[(String, String)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    fn github() -> Webhook {
        config(serde_json::json!({ "secret": "It's a Secret to Everybody" }))
    }

    #[test]
    fn github_documented_example_verifies() {
        // The worked example from GitHub's "Validating webhook deliveries".
        let cfg = github();
        let sig = "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17";
        let h = headers(&[(HMAC_HEADER.to_string(), sig.to_string())]);
        assert_eq!(verify(&cfg, &h, b"Hello, World!", NOW), Ok(()));
    }

    #[test]
    fn hmac_round_trips() {
        let cfg = github();
        let h = headers(&sign(&cfg, BODY, NOW));
        assert_eq!(verify(&cfg, &h, BODY, NOW), Ok(()));
    }

    #[test]
    fn hmac_rejects_wrong_signature_and_tampered_body() {
        let cfg = github();
        let other = config(serde_json::json!({ "secret": "other" }));
        let h = headers(&sign(&other, BODY, NOW));
        assert_eq!(verify(&cfg, &h, BODY, NOW), Err(WebhookError::Mismatch));

        let h = headers(&sign(&cfg, BODY, NOW));
        assert_eq!(
            verify(&cfg, &h, br#"{"action":"closed"}"#, NOW),
            Err(WebhookError::Mismatch)
        );
    }

    #[test]
    fn hmac_rejects_missing_header_and_wrong_prefix() {
        let cfg = github();
        assert_eq!(
            verify(&cfg, &HeaderMap::new(), BODY, NOW),
            Err(WebhookError::MissingHeader(HMAC_HEADER.to_string()))
        );
        let (_, value) = sign(&cfg, BODY, NOW).remove(0);
        let h = headers(&[(HMAC_HEADER.to_string(), value.replace("sha256=", "sha1="))]);
        assert!(matches!(
            verify(&cfg, &h, BODY, NOW),
            Err(WebhookError::Malformed(_))
        ));
        let h = headers(&[(HMAC_HEADER.to_string(), "sha256=zz".to_string())]);
        assert!(matches!(
            verify(&cfg, &h, BODY, NOW),
            Err(WebhookError::Malformed(_))
        ));
    }

    #[test]
    fn empty_secret_refuses_everything() {
        let cfg = config(serde_json::json!({ "secret": "" }));
        let h = headers(&sign(&cfg, BODY, NOW));
        assert_eq!(verify(&cfg, &h, BODY, NOW), Err(WebhookError::EmptySecret));
    }

    #[test]
    fn custom_header_prefix_and_base64() {
        let cfg = config(serde_json::json!({
            "secret": "shpss",
            "header": "X-Shopify-Hmac-Sha256",
            "prefix": "",
            "encoding": "base64",
        }));
        let signed = sign(&cfg, BODY, NOW);
        assert_eq!(signed[0].0, "X-Shopify-Hmac-Sha256");
        assert!(!signed[0].1.starts_with("sha256="));
        assert_eq!(verify(&cfg, &headers(&signed), BODY, NOW), Ok(()));
    }

    #[test]
    fn timestamped_round_trips_and_checks_window() {
        let cfg = config(serde_json::json!({ "secret": "whsec", "scheme": "timestamped" }));
        let h = headers(&sign(&cfg, BODY, NOW));
        assert_eq!(verify(&cfg, &h, BODY, NOW + 299), Ok(()));
        assert_eq!(
            verify(&cfg, &h, BODY, NOW + 301),
            Err(WebhookError::Expired)
        );
        assert_eq!(
            verify(&cfg, &h, BODY, NOW - 301),
            Err(WebhookError::Expired)
        );
        assert_eq!(
            verify(&cfg, &h, b"tampered", NOW),
            Err(WebhookError::Mismatch)
        );
    }

    #[test]
    fn timestamped_accepts_any_v1_and_rejects_moved_timestamp() {
        let cfg = config(serde_json::json!({ "secret": "whsec", "scheme": "timestamped" }));
        let (name, value) = sign(&cfg, BODY, NOW).remove(0);
        let rotated = value.replace("v1=", "v1=00ff,v1=");
        assert_eq!(
            verify(&cfg, &headers(&[(name.clone(), rotated)]), BODY, NOW),
            Ok(())
        );
        // Signing a fresh timestamp onto an old signature breaks it.
        let moved = value.replace(&NOW.to_string(), &(NOW + 10).to_string());
        assert_eq!(
            verify(&cfg, &headers(&[(name.clone(), moved)]), BODY, NOW),
            Err(WebhookError::Mismatch)
        );
        assert!(matches!(
            verify(&cfg, &headers(&[(name, "v1=00".to_string())]), BODY, NOW),
            Err(WebhookError::Malformed(_))
        ));
    }

    #[test]
    fn loco_scheme_matches_loco_notifications() {
        let cfg = config(serde_json::json!({ "secret": "s3cret", "scheme": "loco" }));
        let signed = sign(&cfg, BODY, NOW);
        assert_eq!(verify(&cfg, &headers(&signed), BODY, NOW), Ok(()));
        assert_eq!(
            verify(&cfg, &headers(&signed), BODY, NOW + 1000),
            Err(WebhookError::Expired)
        );
        assert_eq!(
            verify(&cfg, &headers(&signed[..1]), BODY, NOW),
            Err(WebhookError::MissingHeader(
                LOCO_TIMESTAMP_HEADER.to_string()
            ))
        );
    }

    #[cfg(feature = "notifications")]
    #[test]
    fn loco_scheme_accepts_what_notifications_send() {
        let cfg = config(serde_json::json!({ "secret": "s3cret", "scheme": "loco" }));
        let ts = NOW.to_string();
        let sig = crate::notifications::webhook::sign("s3cret", &ts, BODY);
        let h = headers(&[
            (LOCO_SIGNATURE_HEADER.to_string(), sig),
            (LOCO_TIMESTAMP_HEADER.to_string(), ts),
        ]);
        assert_eq!(verify(&cfg, &h, BODY, NOW), Ok(()));
    }
}
