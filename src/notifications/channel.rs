use std::{collections::BTreeMap, fmt};

use crate::app::AppContext;

/// Where a notification goes. Returned by [`super::Notifier::channels`].
///
/// Each channel either names its destination explicitly (`slack_url`,
/// `webhook_url`, `email_to`) or uses the default from the `notifications:`
/// config block (`slack`, `webhook`, `email`). A default that is not
/// configured, or an explicit destination that is empty, is skipped with a
/// warning rather than failing the delivery.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Channel {
    /// A Slack incoming webhook. `None` uses `notifications.slack.webhook_url`.
    Slack { webhook_url: Option<String> },
    /// A JSON `POST`. `None` uses `notifications.webhook.url` together with
    /// its secret and headers.
    Webhook {
        url: Option<String>,
        secret: Option<String>,
        headers: BTreeMap<String, String>,
    },
    /// A Slack incoming webhook looked up by `key` through the app's
    /// [`super::Destinations`] when the job runs. Only the key is queued.
    SlackFor { key: String },
    /// A JSON `POST` whose URL, secret and headers are looked up by `key`
    /// through the app's [`super::Destinations`] when the job runs. Only the
    /// key and the headers set on the channel are queued.
    WebhookFor {
        key: String,
        headers: BTreeMap<String, String>,
    },
    /// Email through the app's mailer. Empty `to` uses
    /// `notifications.email.to`.
    #[cfg(feature = "mailer")]
    Email {
        to: Vec<String>,
        from: Option<String>,
    },
}

// Slack URLs, secrets and header values are credentials: `Debug` hides them.
impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Slack { webhook_url } => f
                .debug_struct("Slack")
                .field("webhook_url", &webhook_url.as_ref().map(|_| "<redacted>"))
                .finish(),
            Self::Webhook {
                url,
                secret,
                headers,
            } => f
                .debug_struct("Webhook")
                .field("url", url)
                .field("secret", &secret.as_ref().map(|_| "<redacted>"))
                .field("headers", &headers.keys().collect::<Vec<_>>())
                .finish(),
            Self::SlackFor { key } => f.debug_struct("SlackFor").field("key", key).finish(),
            Self::WebhookFor { key, headers } => f
                .debug_struct("WebhookFor")
                .field("key", key)
                .field("headers", &headers.keys().collect::<Vec<_>>())
                .finish(),
            #[cfg(feature = "mailer")]
            Self::Email { to, from } => f
                .debug_struct("Email")
                .field("to", to)
                .field("from", from)
                .finish(),
        }
    }
}

impl Channel {
    /// The default Slack webhook from `notifications.slack.webhook_url`.
    #[must_use]
    pub const fn slack() -> Self {
        Self::Slack { webhook_url: None }
    }

    /// A specific Slack incoming-webhook URL, e.g. one stored per team or
    /// per project in your database.
    #[must_use]
    pub fn slack_url(url: impl Into<String>) -> Self {
        Self::Slack {
            webhook_url: Some(url.into()),
        }
    }

    /// The default webhook from `notifications.webhook`.
    #[must_use]
    pub const fn webhook() -> Self {
        Self::Webhook {
            url: None,
            secret: None,
            headers: BTreeMap::new(),
        }
    }

    /// A specific webhook URL, unsigned unless [`Channel::signed`] is added.
    /// The configured secret is never sent to an explicit URL.
    #[must_use]
    pub fn webhook_url(url: impl Into<String>) -> Self {
        Self::Webhook {
            url: Some(url.into()),
            secret: None,
            headers: BTreeMap::new(),
        }
    }

    /// A Slack incoming webhook named by `key` (e.g. a project ID), looked up
    /// through the app's [`super::Destinations`] when the job runs, so the URL
    /// is never written to the queue and a rotated URL is picked up by jobs
    /// already waiting.
    #[must_use]
    pub fn slack_for(key: impl Into<String>) -> Self {
        Self::SlackFor { key: key.into() }
    }

    /// A webhook named by `key`, whose URL, secret and headers are looked up
    /// through the app's [`super::Destinations`] when the job runs. Headers
    /// added with [`Channel::header`] are queued with the job and win over
    /// the looked-up ones.
    #[must_use]
    pub fn webhook_for(key: impl Into<String>) -> Self {
        Self::WebhookFor {
            key: key.into(),
            headers: BTreeMap::new(),
        }
    }

    /// Sign webhook requests with HMAC-SHA256 under `secret` (see
    /// [`super::webhook`]). No effect on other channels.
    #[must_use]
    pub fn signed(mut self, with: impl Into<String>) -> Self {
        if let Self::Webhook { secret, .. } = &mut self {
            *secret = Some(with.into());
        }
        self
    }

    /// Add a request header to a webhook. No effect on other channels.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        if let Self::Webhook { headers, .. } | Self::WebhookFor { headers, .. } = &mut self {
            headers.insert(name.into(), value.into());
        }
        self
    }

    /// Email the default recipients from `notifications.email.to`.
    #[cfg(feature = "mailer")]
    #[must_use]
    pub const fn email() -> Self {
        Self::Email {
            to: Vec::new(),
            from: None,
        }
    }

    /// Email a specific recipient. Chain [`Channel::and_to`] for more.
    #[cfg(feature = "mailer")]
    #[must_use]
    pub fn email_to(to: impl Into<String>) -> Self {
        Self::Email {
            to: vec![to.into()],
            from: None,
        }
    }

    /// Add another email recipient. No effect on other channels.
    #[cfg(feature = "mailer")]
    #[must_use]
    pub fn and_to(mut self, recipient: impl Into<String>) -> Self {
        if let Self::Email { to, .. } = &mut self {
            to.push(recipient.into());
        }
        self
    }

    /// Set the email sender. No effect on other channels.
    #[cfg(feature = "mailer")]
    #[must_use]
    pub fn from(mut self, sender: impl Into<String>) -> Self {
        if let Self::Email { from, .. } = &mut self {
            *from = Some(sender.into());
        }
        self
    }

    /// The channel's name in logs and job payloads.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Slack { .. } | Self::SlackFor { .. } => "slack",
            Self::Webhook { .. } | Self::WebhookFor { .. } => "webhook",
            #[cfg(feature = "mailer")]
            Self::Email { .. } => "email",
        }
    }

    /// Whether this channel has somewhere to deliver to. A keyed channel
    /// needs registered [`super::Destinations`]; whether its key resolves is
    /// only known when the job runs.
    pub(crate) fn is_deliverable(&self, ctx: &AppContext) -> bool {
        let config = ctx.config.notifications.as_ref();
        match self {
            Self::SlackFor { key } | Self::WebhookFor { key, .. } => {
                !key.trim().is_empty() && super::destination::has_destinations(ctx)
            }
            Self::Slack {
                webhook_url: Some(url),
            }
            | Self::Webhook { url: Some(url), .. } => !url.trim().is_empty(),
            Self::Slack { webhook_url: None } => config
                .and_then(|c| c.slack.as_ref())
                .is_some_and(|s| !s.webhook_url.trim().is_empty()),
            Self::Webhook { url: None, .. } => config
                .and_then(|c| c.webhook.as_ref())
                .is_some_and(|w| !w.url.trim().is_empty()),
            #[cfg(feature = "mailer")]
            Self::Email { to, .. } => {
                to.iter().any(|t| !t.trim().is_empty())
                    || config
                        .and_then(|c| c.email.as_ref())
                        .is_some_and(|e| e.to.iter().any(|t| !t.trim().is_empty()))
            }
        }
    }
}
