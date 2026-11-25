//! Destinations looked up when a delivery runs, not when it is enqueued.
//!
//! A Slack URL or webhook secret stored per project in your database is a
//! credential. Passing it to [`Channel::slack_url`](super::Channel::slack_url)
//! or [`Channel::signed`](super::Channel::signed) writes it into the job
//! payload, where it sits in the queue table and goes stale if it is rotated
//! before the job runs. Instead, name the destination with
//! [`Channel::slack_for`](super::Channel::slack_for) or
//! [`Channel::webhook_for`](super::Channel::webhook_for): only the key goes
//! into the job, and the [`Destinations`] registered for the app turn it into
//! a URL, secret and headers each time the job runs.
//!
//! ```rust,ignore
//! use loco_rs::model::ModelError;
//! use loco_rs::notifications::{self, Destination, Destinations};
//!
//! struct ProjectHooks;
//!
//! #[async_trait]
//! impl Destinations for ProjectHooks {
//!     async fn resolve(
//!         &self,
//!         ctx: &AppContext,
//!         channel: &str,
//!         key: &str,
//!     ) -> Result<Option<Destination>> {
//!         let project = match projects::Model::find_by_pid(&ctx.db, key).await {
//!             Ok(project) => project,
//!             // Deleted since: the delivery is dropped, not retried.
//!             Err(ModelError::EntityNotFound) => return Ok(None),
//!             // Anything else (the database is down): the queue retries.
//!             Err(err) => return Err(err.into()),
//!         };
//!         Ok(match channel {
//!             "slack" => project.slack_url.map(Destination::new),
//!             _ => project
//!                 .hook_url
//!                 .map(|url| Destination::new(url).signed_opt(project.hook_secret)),
//!         })
//!     }
//! }
//!
//! // in `Hooks::after_context`
//! notifications::set_destinations(&ctx, ProjectHooks);
//! ```

use std::{collections::BTreeMap, fmt, sync::Arc};

use async_trait::async_trait;

use crate::{app::AppContext, Error, Result};

/// Where one delivery goes, as returned by [`Destinations::resolve`].
#[derive(Clone, PartialEq, Eq)]
pub struct Destination {
    /// The Slack incoming-webhook URL or the webhook URL.
    pub url: String,
    /// Webhook signing secret. Ignored for Slack.
    pub secret: Option<String>,
    /// Webhook request headers. Headers set on the channel itself win.
    /// Ignored for Slack.
    pub headers: BTreeMap<String, String>,
}

// URLs, secrets and header values are credentials: `Debug` hides them.
impl fmt::Debug for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Destination")
            .field("url", &"<redacted>")
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Destination {
    /// A destination at `url`, unsigned and with no extra headers.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            secret: None,
            headers: BTreeMap::new(),
        }
    }

    /// Sign webhook requests under `secret`.
    #[must_use]
    pub fn signed(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Sign webhook requests under `secret` when there is one.
    #[must_use]
    pub fn signed_opt(mut self, secret: Option<String>) -> Self {
        self.secret = secret;
        self
    }

    /// Add a webhook request header.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }
}

/// Resolves the keys of [`Channel::slack_for`](super::Channel::slack_for)
/// and [`Channel::webhook_for`](super::Channel::webhook_for) channels. Register
/// one per app with [`set_destinations`].
#[async_trait]
pub trait Destinations: Send + Sync + 'static {
    /// The destination for `key` on `channel` (`"slack"` or `"webhook"`),
    /// read now, when the delivery runs.
    ///
    /// `Ok(None)` means the destination is gone (the project was deleted,
    /// the hook removed): the delivery is logged and dropped, not retried.
    /// Map "row not found" to `Ok(None)`, not to an error: Loco's
    /// `find_by_pid` finders return `Err(ModelError::EntityNotFound)` for a
    /// missing row, and passing that on with `?` retries a delivery that can
    /// never succeed.
    ///
    /// # Errors
    /// A transient failure (the database is down): the job fails and the
    /// queue retries it.
    async fn resolve(
        &self,
        ctx: &AppContext,
        channel: &str,
        key: &str,
    ) -> Result<Option<Destination>>;
}

#[derive(Clone)]
struct Registered(Arc<dyn Destinations>);

/// Registers the app's [`Destinations`]. Call it once, from
/// `Hooks::after_context`; a later call replaces the earlier one.
pub fn set_destinations(ctx: &AppContext, destinations: impl Destinations) {
    ctx.shared_store.insert(Registered(Arc::new(destinations)));
}

/// Whether `ctx` has [`Destinations`] registered.
#[must_use]
pub fn has_destinations(ctx: &AppContext) -> bool {
    ctx.shared_store.contains::<Registered>()
}

/// Looks `key` up through the registered [`Destinations`]. An empty URL counts
/// as no destination.
pub(super) async fn resolve(
    ctx: &AppContext,
    channel: &str,
    key: &str,
) -> Result<Option<Destination>> {
    let Registered(destinations) = ctx.shared_store.get::<Registered>().ok_or_else(|| {
        Error::Message(format!(
            "{channel} notification for `{key}`: no notification destinations are registered; call notifications::set_destinations in Hooks::after_context"
        ))
    })?;
    let found = destinations.resolve(ctx, channel, key).await?;
    Ok(found.filter(|d| !d.url.trim().is_empty()))
}
