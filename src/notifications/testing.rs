//! Record notifications instead of sending them, and inspect what was sent.
//!
//! The notifications counterpart of the mailer's `stub: true`. Recording is on
//! for a context when `notifications.stub: true` is set in its config (put it
//! in `config/test.yaml`), or after [`record`] is called on it. While it is
//! on, [`Notifier::deliver_later`](super::Notifier::deliver_later) and
//! [`Notifier::deliver_now`](super::Notifier::deliver_now) render every
//! channel, resolve its destination exactly as the worker would, and append a
//! [`Recorded`] delivery instead of enqueueing or sending anything. Recording
//! is synchronous whatever the worker mode, so assertions can follow the call
//! directly.
//!
//! Recordings live in the context's shared store, so tests that build their
//! own context do not see each other's deliveries.
//!
//! ```rust,ignore
//! use loco_rs::notifications::testing;
//!
//! QueuePaused { by: "ana".into() }.deliver_later(&ctx).await;
//!
//! let sent = testing::deliveries(&ctx);
//! assert_eq!(sent.len(), 1);
//! assert_eq!(sent[0].channel, "slack");
//! assert!(sent[0].text().contains("Queue paused by ana"));
//! testing::clear(&ctx);
//! ```

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};

use serde::Serialize;
use serde_json::Value;

use super::{slack_url, webhook_destination, Delivery, Target};
use crate::{app::AppContext, Result};

/// One delivery as it would have been sent.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Recorded {
    /// The delivery ID (`X-Loco-Delivery` for webhooks).
    pub id: String,
    /// The event name (`X-Loco-Event` for webhooks).
    pub event: String,
    /// `slack`, `webhook` or `email`.
    pub channel: String,
    /// The resolved destination: the Slack or webhook URL, or the email
    /// recipients.
    pub to: Vec<String>,
    /// What would have been sent: the Slack or webhook JSON body, or, for
    /// email, `{from, subject, text, html}`.
    pub payload: Value,
    /// Webhook request headers from the channel and config (not the
    /// `X-Loco-*` ones). Empty for other channels.
    pub headers: BTreeMap<String, String>,
    /// Whether a webhook delivery would have been signed.
    pub signed: bool,
}

impl Recorded {
    /// The human-readable text of the delivery, to assert on regardless of
    /// channel: Slack's `text`, the webhook `title` and `body`, or the email
    /// subject and text body.
    #[must_use]
    pub fn text(&self) -> String {
        let field = |name: &str| {
            self.payload
                .get(name)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        match self.channel.as_str() {
            "slack" => field("text"),
            "webhook" => format!("{}\n{}", field("title"), field("body")),
            _ => format!("{}\n{}", field("subject"), field("text")),
        }
    }
}

/// The recording of one context. Cloning shares it.
#[derive(Debug, Clone, Default)]
pub(super) struct Recorder(Arc<Mutex<Vec<Recorded>>>);

impl Recorder {
    fn lock(&self) -> MutexGuard<'_, Vec<Recorded>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Resolves `delivery` as the worker would and records it.
    pub(super) fn record(&self, ctx: &AppContext, delivery: Delivery) -> Result<()> {
        let config = ctx.config.notifications.as_ref();
        let channel = delivery.target.name().to_string();
        let (to, payload, headers, signed) = match delivery.target {
            Target::Slack {
                webhook_url,
                payload,
            } => (
                vec![slack_url(config, webhook_url)?],
                payload,
                BTreeMap::new(),
                false,
            ),
            Target::Webhook {
                url,
                secret,
                headers,
                payload,
            } => {
                let (url, secret, headers) = webhook_destination(config, url, secret, headers)?;
                (vec![url], payload, headers, secret.is_some())
            }
            #[cfg(feature = "mailer")]
            Target::Email { to, from, content } => {
                let (to, from) = super::email::resolve(ctx, to, from)?;
                let payload = serde_json::json!({
                    "from": from,
                    "subject": content.subject,
                    "text": content.text,
                    "html": content.html,
                });
                (to, payload, BTreeMap::new(), false)
            }
        };
        self.lock().push(Recorded {
            id: delivery.id,
            event: delivery.event,
            channel,
            to,
            payload,
            headers,
            signed,
        });
        Ok(())
    }
}

/// Serializes the first insert of a context's recorder.
static INIT: Mutex<()> = Mutex::new(());

fn get_or_insert(ctx: &AppContext) -> Recorder {
    let _guard = INIT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(recorder) = ctx.shared_store.get::<Recorder>() {
        return recorder;
    }
    let recorder = Recorder::default();
    ctx.shared_store.insert(recorder.clone());
    recorder
}

/// The context's recorder when recording is on.
pub(super) fn recorder(ctx: &AppContext) -> Option<Recorder> {
    if let Some(recorder) = ctx.shared_store.get::<Recorder>() {
        return Some(recorder);
    }
    ctx.config
        .notifications
        .as_ref()
        .is_some_and(|n| n.stub)
        .then(|| get_or_insert(ctx))
}

/// Turns recording on for `ctx`, as `notifications.stub: true` does. Has no
/// effect if it is already on.
pub fn record(ctx: &AppContext) {
    get_or_insert(ctx);
}

/// Whether deliveries on `ctx` are recorded instead of sent.
#[must_use]
pub fn is_recording(ctx: &AppContext) -> bool {
    recorder(ctx).is_some()
}

/// Every delivery recorded on `ctx` so far, oldest first. Empty when
/// recording is off.
#[must_use]
pub fn deliveries(ctx: &AppContext) -> Vec<Recorded> {
    recorder(ctx).map(|r| r.lock().clone()).unwrap_or_default()
}

/// Forgets the deliveries recorded on `ctx`; recording stays on.
pub fn clear(ctx: &AppContext) {
    if let Some(recorder) = recorder(ctx) {
        recorder.lock().clear();
    }
}
