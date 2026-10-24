use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Notifications configuration: the default destination of each channel and
/// how deliveries are retried.
///
/// Every block is optional. A notifier that asks for a channel's default
/// destination (`Channel::slack()`, `Channel::webhook()`, `Channel::email()`)
/// while that block is absent, or its URL is empty, skips the channel with a
/// warning instead of failing — so an unset `SLACK_WEBHOOK_URL` in development
/// turns Slack off rather than breaking the request.
///
/// Secrets read from here (the Slack URL, the webhook signing secret) are
/// resolved when the job runs, so they are never written into the job queue.
///
/// ```yaml
/// notifications:
///   slack:
///     webhook_url: <%= get_env(name="SLACK_WEBHOOK_URL", default="") %>
///   webhook:
///     url: https://example.com/hooks/loco
///     secret: <%= get_env(name="WEBHOOK_SECRET", default="") %>
///   email:
///     to: [ops@example.com]
///   max_retries: 5
///   retry_backoff_secs: 10
///   timeout_secs: 10
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Notifications {
    /// Default Slack incoming webhook.
    #[serde(default)]
    pub slack: Option<SlackNotifications>,
    /// Default outgoing JSON webhook.
    #[serde(default)]
    pub webhook: Option<WebhookNotifications>,
    /// Default email recipients. Sent through the app's mailer.
    #[serde(default)]
    pub email: Option<EmailNotifications>,
    /// Retries after a delivery's first failure (Postgres and `SQLite`
    /// queues; other providers deliver once).
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Delay before the first retry, in seconds; each retry doubles it.
    #[serde(default = "default_retry_backoff_secs")]
    pub retry_backoff_secs: u64,
    /// HTTP timeout for Slack and webhook deliveries, in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            slack: None,
            webhook: None,
            email: None,
            max_retries: default_max_retries(),
            retry_backoff_secs: default_retry_backoff_secs(),
            timeout_secs: default_timeout_secs(),
        }
    }
}

/// Default Slack destination.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SlackNotifications {
    /// A Slack incoming-webhook URL. Empty means "not configured".
    #[serde(default)]
    pub webhook_url: String,
}

/// Default webhook destination.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WebhookNotifications {
    /// Where to POST the JSON payload. Empty means "not configured".
    #[serde(default)]
    pub url: String,
    /// HMAC-SHA256 signing secret. Empty or absent sends unsigned requests.
    #[serde(default)]
    pub secret: Option<String>,
    /// Extra request headers, e.g. an `Authorization` token.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

/// Default email destination.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EmailNotifications {
    /// Recipients. Empty means "not configured".
    #[serde(default)]
    pub to: Vec<String>,
    /// Sender; falls back to the mailer default.
    #[serde(default)]
    pub from: Option<String>,
}

const fn default_max_retries() -> u32 {
    5
}

const fn default_retry_backoff_secs() -> u64 {
    10
}

const fn default_timeout_secs() -> u64 {
    10
}
