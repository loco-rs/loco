//! Notifications: define a notifier once, deliver it to Slack, a webhook and
//! email in the background.
//!
//! A notifier is any `Serialize` struct that implements [`Notifier`]: it
//! says what the notification reads like ([`Notifier::message`]) and where it
//! goes ([`Notifier::channels`]). [`Notifier::deliver_later`] renders one
//! [`Delivery`] per channel and hands each to the built-in
//! [`NotificationWorker`] on the `notifications` queue, so a slow or broken
//! channel never blocks or fails the request that triggered it, and each
//! channel is retried on its own (a Slack outage does not re-send the email).
//!
//! ```rust,ignore
//! use loco_rs::prelude::*;
//! use loco_rs::notifications::{Channel, Message, Notifier};
//!
//! #[derive(Serialize)]
//! pub struct QueuePaused {
//!     pub by: String,
//!     pub reason: String,
//! }
//!
//! impl Notifier for QueuePaused {
//!     fn message(&self) -> Message {
//!         Message::new(format!("Queue paused by {}", self.by)).body(&self.reason)
//!     }
//!
//!     fn channels(&self, _ctx: &AppContext) -> Vec<Channel> {
//!         vec![Channel::slack(), Channel::webhook()]
//!     }
//! }
//!
//! QueuePaused { by: "ana".into(), reason: "deploy freeze".into() }
//!     .deliver_later(&ctx)
//!     .await;
//! ```

mod channel;
#[cfg(feature = "mailer")]
pub mod email;
pub mod slack;
pub mod testing;
pub mod webhook;

use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;
pub use channel::Channel;
#[cfg(feature = "mailer")]
pub use email::EmailContent;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    app::AppContext,
    bgworker::{BackgroundWorker, Queue, RetryPolicy},
    config::Notifications,
    Error, Result,
};

/// The queue notification deliveries are enqueued on.
pub const QUEUE: &str = "notifications";

/// One `name: value` line of a [`Message`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub value: String,
}

/// What a notification says, independent of channel. Each channel renders
/// it its own way; override the `*_payload` / `email` methods on
/// [`Notifier`] for full control.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub title: String,
    pub body: Option<String>,
    pub url: Option<String>,
    pub fields: Vec<Field>,
}

impl Message {
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    #[must_use]
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push(Field {
            name: name.into(),
            value: value.into(),
        });
        self
    }
}

/// What a single [`Notifier::deliver_later`] call did, per channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// Handed to the queue (or, in `ForegroundBlocking` mode, sent).
    pub accepted: usize,
    /// Not configured: a default channel with no config, or an empty URL.
    pub skipped: usize,
    /// Could not be rendered or enqueued (or, in `ForegroundBlocking` mode,
    /// failed to send). Logged; never returned as an error.
    pub failed: usize,
}

/// A notification type. Implement it on a `Serialize` struct holding the
/// event's data; that data is also sent as `data` in webhook payloads.
#[async_trait]
pub trait Notifier: Serialize + Send + Sync + Sized {
    /// The event name sent to webhooks (`X-Loco-Event`). Defaults to the
    /// type name in `snake_case`, e.g. `QueuePaused` → `queue_paused`.
    #[must_use]
    fn event() -> String {
        use heck::ToSnakeCase;
        let name = std::any::type_name::<Self>();
        name.rsplit("::").next().unwrap_or(name).to_snake_case()
    }

    /// The notification's content.
    fn message(&self) -> Message;

    /// Where to deliver it. Decided per notification, so it can depend on
    /// the data (a per-project Slack URL) or on `ctx` (an environment).
    fn channels(&self, ctx: &AppContext) -> Vec<Channel>;

    /// Background-queue priority for the deliveries.
    #[must_use]
    fn priority() -> Option<i32> {
        None
    }

    /// The JSON sent to a Slack incoming webhook. Defaults to
    /// [`slack::payload`].
    fn slack_payload(&self, message: &Message) -> Value {
        slack::payload(message)
    }

    /// The JSON body of a webhook. Receives the default payload (`id`,
    /// `event`, `created_at`, `title`, `body`, `url`, `fields`, and `data`
    /// holding this notifier serialized) and returns what is sent.
    fn webhook_payload(&self, payload: Value) -> Value {
        payload
    }

    /// The email's parts. Defaults to [`EmailContent::from_message`]; use
    /// [`EmailContent::from_template`] to render mailer templates.
    ///
    /// # Errors
    /// When rendering fails; that channel is then reported as failed.
    #[cfg(feature = "mailer")]
    fn email(&self, message: &Message) -> Result<EmailContent> {
        Ok(EmailContent::from_message(message))
    }

    /// Renders one [`Delivery`] per deliverable channel.
    ///
    /// # Errors
    /// When the notifier cannot be serialized or a channel fails to render.
    fn deliveries(&self, ctx: &AppContext) -> Result<(Vec<Delivery>, usize)> {
        let config = ctx.config.notifications.as_ref();
        let message = self.message();
        let event = Self::event();
        let mut deliveries = Vec::new();
        let mut skipped = 0;
        for channel in self.channels(ctx) {
            if !channel.is_deliverable(config) {
                tracing::warn!(
                    event,
                    channel = channel.name(),
                    "notification channel not configured; skipping"
                );
                skipped += 1;
                continue;
            }
            let id = uuid::Uuid::new_v4().to_string();
            let target = match channel {
                Channel::Slack { webhook_url } => Target::Slack {
                    webhook_url,
                    payload: self.slack_payload(&message),
                },
                Channel::Webhook {
                    url,
                    secret,
                    headers,
                } => Target::Webhook {
                    url,
                    secret,
                    headers,
                    payload: self.webhook_payload(json!({
                        "id": id,
                        "event": event,
                        "created_at": chrono::Utc::now().to_rfc3339(),
                        "title": message.title,
                        "body": message.body,
                        "url": message.url,
                        "fields": message.fields,
                        "data": serde_json::to_value(self)?,
                    })),
                },
                #[cfg(feature = "mailer")]
                Channel::Email { to, from } => Target::Email {
                    to,
                    from,
                    content: self.email(&message)?,
                },
            };
            deliveries.push(Delivery {
                id,
                event: event.clone(),
                target,
            });
        }
        Ok((deliveries, skipped))
    }

    /// Delivers to every channel in the background, through the
    /// `notifications` queue. Never fails: problems are logged and counted
    /// in the returned [`Report`], so calling this cannot break a request.
    async fn deliver_later(&self, ctx: &AppContext) -> Report {
        let event = Self::event();
        let (deliveries, skipped) = match self.deliveries(ctx) {
            Ok(rendered) => rendered,
            Err(err) => {
                tracing::error!(
                    event,
                    err = err.to_string(),
                    "could not render notification"
                );
                return Report {
                    failed: 1,
                    ..Report::default()
                };
            }
        };
        let mut report = Report {
            skipped,
            ..Report::default()
        };
        if let Some(recorder) = testing::recorder(ctx) {
            for delivery in deliveries {
                match recorder.record(ctx, delivery) {
                    Ok(()) => report.accepted += 1,
                    Err(err) => {
                        tracing::error!(event, err = err.to_string(), "notification failed");
                        report.failed += 1;
                    }
                }
            }
            return report;
        }
        for delivery in deliveries {
            let channel = delivery.target.name();
            match NotificationWorker::perform_later_with_priority(ctx, delivery, Self::priority())
                .await
            {
                Ok(_) => report.accepted += 1,
                Err(err) => {
                    tracing::error!(
                        event,
                        channel,
                        err = err.to_string(),
                        "notification delivery failed"
                    );
                    report.failed += 1;
                }
            }
        }
        report
    }

    /// Delivers to every channel now, in this task, bypassing the queue and
    /// its retries. For tasks, scripts and tests.
    ///
    /// # Errors
    /// After trying every channel, when any of them failed.
    async fn deliver_now(&self, ctx: &AppContext) -> Result<Report> {
        let (deliveries, skipped) = self.deliveries(ctx)?;
        let worker = NotificationWorker::build(ctx);
        let mut report = Report {
            skipped,
            ..Report::default()
        };
        let mut errors = Vec::new();
        let recorder = testing::recorder(ctx);
        for delivery in deliveries {
            let channel = delivery.target.name();
            let sent = match &recorder {
                Some(recorder) => recorder.record(ctx, delivery),
                None => worker.perform(delivery).await,
            };
            match sent {
                Ok(()) => report.accepted += 1,
                Err(err) => {
                    report.failed += 1;
                    errors.push(format!("{channel}: {err}"));
                }
            }
        }
        if errors.is_empty() {
            Ok(report)
        } else {
            Err(Error::Message(format!(
                "notification delivery failed: {}",
                errors.join("; ")
            )))
        }
    }
}

/// A rendered notification for one channel: the job payload of
/// [`NotificationWorker`]. Secrets from config are not in it; they are read
/// when the job runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delivery {
    /// Stable across retries; sent to webhooks as `X-Loco-Delivery`.
    pub id: String,
    pub event: String,
    pub target: Target,
}

/// A rendered destination. `None` destinations are resolved from the
/// `notifications:` config when the job runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "channel", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Target {
    Slack {
        webhook_url: Option<String>,
        payload: Value,
    },
    Webhook {
        url: Option<String>,
        secret: Option<String>,
        #[serde(default)]
        headers: BTreeMap<String, String>,
        payload: Value,
    },
    #[cfg(feature = "mailer")]
    Email {
        to: Vec<String>,
        from: Option<String>,
        content: EmailContent,
    },
}

impl Target {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Slack { .. } => "slack",
            Self::Webhook { .. } => "webhook",
            #[cfg(feature = "mailer")]
            Self::Email { .. } => "email",
        }
    }
}

/// Sends one [`Delivery`]. Registered automatically at boot when the
/// `notifications` feature is on; a failed send returns an error so the
/// queue retries it (`notifications.max_retries`, Postgres and `SQLite`).
pub struct NotificationWorker {
    ctx: AppContext,
    http: reqwest::Client,
}

#[async_trait]
impl BackgroundWorker<Delivery> for NotificationWorker {
    fn queue() -> Option<String> {
        Some(QUEUE.to_string())
    }

    fn build(ctx: &AppContext) -> Self {
        let timeout = ctx
            .config
            .notifications
            .as_ref()
            .map_or(10, |n| n.timeout_secs);
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .unwrap_or_default();
        Self {
            ctx: ctx.clone(),
            http,
        }
    }

    async fn perform(&self, delivery: Delivery) -> Result<()> {
        let config = self.ctx.config.notifications.as_ref();
        match delivery.target {
            Target::Slack {
                webhook_url,
                payload,
            } => {
                let url = slack_url(config, webhook_url)?;
                slack::send(&self.http, &url, &payload).await
            }
            Target::Webhook {
                url,
                secret,
                headers,
                payload,
            } => {
                let (url, secret, headers) = webhook_destination(config, url, secret, headers)?;
                webhook::send(
                    &self.http,
                    webhook::Request {
                        url: &url,
                        secret: secret.as_deref(),
                        headers: &headers,
                        event: &delivery.event,
                        delivery_id: &delivery.id,
                        payload: &payload,
                    },
                )
                .await
            }
            #[cfg(feature = "mailer")]
            Target::Email { to, from, content } => email::send(&self.ctx, to, from, content).await,
        }
    }
}

/// The Slack URL of a delivery: its own, else `notifications.slack`.
fn slack_url(config: Option<&Notifications>, webhook_url: Option<String>) -> Result<String> {
    webhook_url
        .or_else(|| {
            config
                .and_then(|c| c.slack.as_ref())
                .map(|s| s.webhook_url.clone())
        })
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| {
            Error::string("slack notification: notifications.slack.webhook_url is not set")
        })
}

/// The URL, secret and headers of a webhook delivery. An explicit URL keeps
/// its own secret; the default one takes `notifications.webhook`'s secret and
/// headers.
fn webhook_destination(
    config: Option<&Notifications>,
    url: Option<String>,
    secret: Option<String>,
    mut headers: BTreeMap<String, String>,
) -> Result<(String, Option<String>, BTreeMap<String, String>)> {
    let (url, secret) = match url {
        Some(url) => (url, secret),
        None => {
            let defaults = config.and_then(|c| c.webhook.as_ref()).ok_or_else(|| {
                Error::string("webhook notification: notifications.webhook is not set")
            })?;
            for (name, value) in &defaults.headers {
                headers.entry(name.clone()).or_insert_with(|| value.clone());
            }
            (
                defaults.url.clone(),
                secret.or_else(|| defaults.secret.clone()),
            )
        }
    };
    if url.trim().is_empty() {
        return Err(Error::string("webhook notification: no URL"));
    }
    let secret = secret.filter(|s| !s.is_empty());
    Ok((url, secret, headers))
}

/// Registers [`NotificationWorker`] with the retry policy from config.
pub(crate) async fn register_worker(ctx: &AppContext, queue: &Queue) -> Result<()> {
    let config = ctx.config.notifications.clone().unwrap_or_default();
    queue
        .register_with_retry_policy(
            NotificationWorker::build(ctx),
            RetryPolicy {
                max_retries: config.max_retries,
                backoff: Duration::from_secs(config.retry_backoff_secs),
            },
        )
        .await
}

/// A transport error without its URL: Slack webhook URLs are secrets, and
/// reqwest puts the URL in its error text.
pub(crate) fn http_error(channel: &str, err: reqwest::Error) -> Error {
    Error::Message(format!("{channel} delivery failed: {}", err.without_url()))
}

#[cfg(test)]
mod tests;
