use serde::{Deserialize, Serialize};

/// An incoming webhook source whose requests are signed with a shared secret,
/// verified by the `SignedWebhook` extractor.
///
/// Each entry under `webhooks:` is keyed by the name the extractor's source
/// type returns from `WebhookSource::NAME`:
///
/// ```yaml
/// webhooks:
///   github:
///     secret: <%= get_env(name="GITHUB_WEBHOOK_SECRET", default="") %>
///     # scheme: hmac (default), header X-Hub-Signature-256, prefix "sha256="
///   stripe:
///     secret: <%= get_env(name="STRIPE_WEBHOOK_SECRET", default="") %>
///     scheme: timestamped
///     tolerance_secs: 300
///   shopify:
///     secret: <%= get_env(name="SHOPIFY_WEBHOOK_SECRET", default="") %>
///     header: X-Shopify-Hmac-Sha256
///     prefix: ""
///     encoding: base64
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Webhook {
    /// Shared signing secret. An empty secret refuses every request: HMAC
    /// accepts an empty key, which would let anyone sign a payload.
    pub secret: String,
    /// How the signature is laid out. Defaults to [`WebhookScheme::Hmac`].
    #[serde(default)]
    pub scheme: WebhookScheme,
    /// Header carrying the signature. Defaults per scheme:
    /// `X-Hub-Signature-256` (`hmac`), `Stripe-Signature` (`timestamped`),
    /// `X-Loco-Signature` (`loco`).
    #[serde(default)]
    pub header: Option<String>,
    /// Text before the digest in the `hmac` and `loco` schemes. Defaults to
    /// `sha256=`; set `""` for none.
    #[serde(default)]
    pub prefix: Option<String>,
    /// How the digest is written. Defaults to `hex`.
    #[serde(default)]
    pub encoding: SignatureEncoding,
    /// For schemes that sign a timestamp, how old (or how far in the future)
    /// a request may be, in seconds. Defaults to 300.
    #[serde(default = "default_tolerance_secs")]
    pub tolerance_secs: u64,
}

const fn default_tolerance_secs() -> u64 {
    300
}

/// The signature layouts `SignedWebhook` understands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookScheme {
    /// `HMAC-SHA256(secret, body)` in one header, after an optional prefix.
    /// GitHub (`X-Hub-Signature-256: sha256=<hex>`), Shopify, and most
    /// others.
    #[default]
    Hmac,
    /// `t=<unix secs>,v1=<hex>[,v1=<hex>]` in one header, signing
    /// `"{t}.{body}"`, with a replay window. Stripe's layout.
    Timestamped,
    /// What Loco's own notification webhooks send: `X-Loco-Timestamp` and
    /// `X-Loco-Signature: sha256=<hex>` over `"{timestamp}.{body}"`.
    Loco,
}

/// How the digest bytes are written in the header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureEncoding {
    /// Lowercase or uppercase hex.
    #[default]
    Hex,
    /// Standard base64 with padding.
    Base64,
}
