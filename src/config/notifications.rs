use std::{collections::BTreeMap, fmt};

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
///   stub: false # true in config/test.yaml: record instead of sending
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
    /// Record deliveries instead of sending them, for tests. Read them back
    /// with `loco_rs::notifications::testing::deliveries(&ctx)`.
    #[serde(default)]
    pub stub: bool,
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
            stub: false,
        }
    }
}

/// Default Slack destination.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct SlackNotifications {
    /// A Slack incoming-webhook URL. Empty means "not configured".
    #[serde(default)]
    pub webhook_url: String,
}

/// Default webhook destination.
#[derive(Clone, Default, Deserialize, Serialize)]
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

// Slack URLs and signing secrets are credentials, so `Debug` (and with it
// `{:?}` of the whole config) never prints them.
impl fmt::Debug for SlackNotifications {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlackNotifications")
            .field("webhook_url", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for WebhookNotifications {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebhookNotifications")
            .field("url", &self.url)
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .finish()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_secrets() {
        let config = Notifications {
            slack: Some(SlackNotifications {
                webhook_url: "https://hooks.slack.test/T0/SLACKSECRET".into(),
            }),
            webhook: Some(WebhookNotifications {
                url: "https://example.com/hook".into(),
                secret: Some("WEBHOOKSECRET".into()),
                headers: BTreeMap::from([("Authorization".into(), "Bearer HEADERSECRET".into())]),
            }),
            ..Notifications::default()
        };
        let dump = format!("{config:?}");
        for secret in ["SLACKSECRET", "WEBHOOKSECRET", "HEADERSECRET"] {
            assert!(!dump.contains(secret), "{secret} leaked: {dump}");
        }
        assert!(dump.contains("<redacted>"));
    }
}
