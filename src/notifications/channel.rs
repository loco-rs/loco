use std::collections::BTreeMap;

use crate::config::Notifications;

/// Where a notification goes. Returned by [`super::Notifier::channels`].
///
/// Each channel either names its destination explicitly (`slack_url`,
/// `webhook_url`, `email_to`) or uses the default from the `notifications:`
/// config block (`slack`, `webhook`, `email`). A default that is not
/// configured, or an explicit destination that is empty, is skipped with a
/// warning rather than failing the delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Email through the app's mailer. Empty `to` uses
    /// `notifications.email.to`.
    #[cfg(feature = "mailer")]
    Email {
        to: Vec<String>,
        from: Option<String>,
    },
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
        if let Self::Webhook { headers, .. } = &mut self {
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
            Self::Slack { .. } => "slack",
            Self::Webhook { .. } => "webhook",
            #[cfg(feature = "mailer")]
            Self::Email { .. } => "email",
        }
    }

    /// Whether this channel has somewhere to deliver to.
    pub(crate) fn is_deliverable(&self, config: Option<&Notifications>) -> bool {
        match self {
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
