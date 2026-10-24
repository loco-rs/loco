//! Outgoing JSON webhooks, optionally signed.
//!
//! Every request is a `POST` with `Content-Type: application/json` and these
//! headers:
//!
//! | header | value |
//! |---|---|
//! | `X-Loco-Event` | the notifier's [`super::Notifier::event`] name |
//! | `X-Loco-Delivery` | a delivery ID, stable across retries (dedupe on it) |
//! | `X-Loco-Timestamp` | Unix seconds when this attempt was sent |
//! | `X-Loco-Signature` | `sha256=<hex>`, only when the channel has a secret |
//!
//! The signature is HMAC-SHA256 under the shared secret over
//! `"{timestamp}.{raw body}"`. Binding the timestamp lets a receiver reject
//! replays: check the signature with [`verify_signature`], then refuse a
//! timestamp older than a few minutes.

use std::collections::BTreeMap;

use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

use super::http_error;
use crate::Result;

/// Header carrying the event name.
pub const EVENT_HEADER: &str = "X-Loco-Event";
/// Header carrying the delivery ID.
pub const DELIVERY_HEADER: &str = "X-Loco-Delivery";
/// Header carrying the attempt's Unix timestamp.
pub const TIMESTAMP_HEADER: &str = "X-Loco-Timestamp";
/// Header carrying the `sha256=<hex>` signature.
pub const SIGNATURE_HEADER: &str = "X-Loco-Signature";

type HmacSha256 = Hmac<Sha256>;

fn mac(secret: &str, timestamp: &str, body: &[u8]) -> HmacSha256 {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    mac
}

/// The `X-Loco-Signature` value for `body` sent at `timestamp`.
#[must_use]
pub fn sign(secret: &str, timestamp: &str, body: &[u8]) -> String {
    let digest = mac(secret, timestamp, body).finalize().into_bytes();
    let mut hex = String::with_capacity(7 + digest.len() * 2);
    hex.push_str("sha256=");
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Checks a received `X-Loco-Signature` header in constant time. For apps
/// that receive Loco webhooks; it does not check the timestamp's age.
#[must_use]
pub fn verify_signature(secret: &str, timestamp: &str, body: &[u8], signature: &str) -> bool {
    let Some(hex) = signature.strip_prefix("sha256=") else {
        return false;
    };
    if hex.len() % 2 != 0 {
        return false;
    }
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect();
    bytes.is_some_and(|bytes| mac(secret, timestamp, body).verify_slice(&bytes).is_ok())
}

pub(super) struct Request<'a> {
    pub url: &'a str,
    pub secret: Option<&'a str>,
    pub headers: &'a BTreeMap<String, String>,
    pub event: &'a str,
    pub delivery_id: &'a str,
    pub payload: &'a Value,
}

pub(super) async fn send(http: &reqwest::Client, req: Request<'_>) -> Result<()> {
    let body = serde_json::to_vec(req.payload)?;
    let timestamp = chrono::Utc::now().timestamp().to_string();

    let mut builder = http
        .post(req.url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::USER_AGENT, "loco-notifications")
        .header(EVENT_HEADER, req.event)
        .header(DELIVERY_HEADER, req.delivery_id)
        .header(TIMESTAMP_HEADER, &timestamp);
    if let Some(secret) = req.secret.filter(|s| !s.is_empty()) {
        builder = builder.header(SIGNATURE_HEADER, sign(secret, &timestamp, &body));
    }
    for (name, value) in req.headers {
        builder = builder.header(name, value);
    }

    let response = builder
        .body(body)
        .send()
        .await
        .map_err(|e| http_error("webhook", e))?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(crate::Error::Message(format!("webhook answered {status}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_round_trips_and_rejects_tampering() {
        let sig = sign("s3cret", "1700000000", b"{\"a\":1}");
        assert!(sig.starts_with("sha256="));
        assert_eq!(sig.len(), 7 + 64);
        assert!(verify_signature("s3cret", "1700000000", b"{\"a\":1}", &sig));
        assert!(!verify_signature(
            "s3cret",
            "1700000001",
            b"{\"a\":1}",
            &sig
        ));
        assert!(!verify_signature(
            "s3cret",
            "1700000000",
            b"{\"a\":2}",
            &sig
        ));
        assert!(!verify_signature("other", "1700000000", b"{\"a\":1}", &sig));
        assert!(!verify_signature(
            "s3cret",
            "1700000000",
            b"{\"a\":1}",
            "sha256=zz"
        ));
        assert!(!verify_signature(
            "s3cret",
            "1700000000",
            b"{\"a\":1}",
            "garbage"
        ));
    }
}
