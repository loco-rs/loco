//! Live page updates in the style of Turbo Streams.
//!
//! Two halves, matching Rails' `turbo-rails`:
//!
//! - [`TurboStream`] builds `<turbo-stream>` elements and answers a request
//!   with them (`text/vnd.turbo-stream.html`), the equivalent of
//!   `respond_to { |f| f.turbo_stream }` and `turbo_stream.append(...)`.
//! - [`Broadcaster`] fans those elements out to browsers subscribed to a named
//!   stream over Server-Sent Events, the equivalent of `broadcast_append_to`.
//!   A page subscribes with `<turbo-stream-source src="/streams/...">`.
//!
//! The broadcaster is in-process: a message reaches subscribers connected to
//! the same server process only.
//!
//! ```rust
//! use loco_rs::controller::turbo::{TurboStream, TurboStreams};
//!
//! let html = TurboStreams::from(vec![
//!     TurboStream::append("posts", "<li id=\"post_1\">Hello</li>"),
//!     TurboStream::remove("flash"),
//! ])
//! .to_html();
//! assert!(html.starts_with(r#"<turbo-stream action="append" target="posts">"#));
//! ```

use std::{convert::Infallible, fmt, sync::Arc, time::Duration};

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderMap, HeaderValue},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
};
use dashmap::DashMap;
use futures_util::{stream, Stream, StreamExt};
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::{controller::views::ViewRenderer, Error, Result};

/// The media type of a Turbo Stream response.
pub const CONTENT_TYPE: &str = "text/vnd.turbo-stream.html";

/// What a `<turbo-stream>` element does to its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Append the template to the target's children.
    Append,
    /// Prepend the template to the target's children.
    Prepend,
    /// Replace the target element itself.
    Replace,
    /// Replace the target's children.
    Update,
    /// Remove the target element.
    Remove,
    /// Insert the template before the target.
    Before,
    /// Insert the template after the target.
    After,
    /// Ask the page to refresh itself.
    Refresh,
}

impl Action {
    /// The value of the `action` attribute.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Append => "append",
            Self::Prepend => "prepend",
            Self::Replace => "replace",
            Self::Update => "update",
            Self::Remove => "remove",
            Self::Before => "before",
            Self::After => "after",
            Self::Refresh => "refresh",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    None,
    Id(String),
    Selector(String),
}

/// One `<turbo-stream>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurboStream {
    action: Action,
    target: Target,
    template: Option<String>,
}

impl TurboStream {
    /// A stream element acting on the element whose id is `target`.
    #[must_use]
    pub fn new(action: Action, target: impl Into<String>) -> Self {
        Self {
            action,
            target: Target::Id(target.into()),
            template: None,
        }
    }

    /// A stream element acting on every element matching the CSS `selector`
    /// (the `targets` attribute).
    #[must_use]
    pub fn new_all(action: Action, selector: impl Into<String>) -> Self {
        Self {
            action,
            target: Target::Selector(selector.into()),
            template: None,
        }
    }

    /// Set the HTML placed inside `<template>`. It is inserted as is, so it
    /// must already be escaped, as a rendered view is.
    #[must_use]
    pub fn template(mut self, html: impl Into<String>) -> Self {
        self.template = Some(html.into());
        self
    }

    /// Render the view at `key` and use it as the template.
    ///
    /// # Errors
    ///
    /// When the view fails to render.
    pub fn render<V: ViewRenderer, S: Serialize>(self, v: &V, key: &str, data: S) -> Result<Self> {
        Ok(self.template(v.render(key, data)?))
    }

    /// `turbo_stream.append(target, html)`
    #[must_use]
    pub fn append(target: impl Into<String>, html: impl Into<String>) -> Self {
        Self::new(Action::Append, target).template(html)
    }

    /// `turbo_stream.prepend(target, html)`
    #[must_use]
    pub fn prepend(target: impl Into<String>, html: impl Into<String>) -> Self {
        Self::new(Action::Prepend, target).template(html)
    }

    /// `turbo_stream.replace(target, html)`
    #[must_use]
    pub fn replace(target: impl Into<String>, html: impl Into<String>) -> Self {
        Self::new(Action::Replace, target).template(html)
    }

    /// `turbo_stream.update(target, html)`
    #[must_use]
    pub fn update(target: impl Into<String>, html: impl Into<String>) -> Self {
        Self::new(Action::Update, target).template(html)
    }

    /// `turbo_stream.before(target, html)`
    #[must_use]
    pub fn before(target: impl Into<String>, html: impl Into<String>) -> Self {
        Self::new(Action::Before, target).template(html)
    }

    /// `turbo_stream.after(target, html)`
    #[must_use]
    pub fn after(target: impl Into<String>, html: impl Into<String>) -> Self {
        Self::new(Action::After, target).template(html)
    }

    /// `turbo_stream.remove(target)`
    #[must_use]
    pub fn remove(target: impl Into<String>) -> Self {
        Self::new(Action::Remove, target)
    }

    /// `turbo_stream.refresh`: the page reloads itself (Turbo 8 morphing).
    #[must_use]
    pub const fn refresh() -> Self {
        Self {
            action: Action::Refresh,
            target: Target::None,
            template: None,
        }
    }

    /// The element as HTML.
    #[must_use]
    pub fn to_html(&self) -> String {
        let mut out = format!(r#"<turbo-stream action="{}""#, self.action);
        match &self.target {
            Target::None => {}
            Target::Id(id) => {
                out.push_str(r#" target=""#);
                escape_attr(&mut out, id);
                out.push('"');
            }
            Target::Selector(sel) => {
                out.push_str(r#" targets=""#);
                escape_attr(&mut out, sel);
                out.push('"');
            }
        }
        out.push('>');
        if let Some(html) = &self.template {
            out.push_str("<template>");
            out.push_str(html);
            out.push_str("</template>");
        }
        out.push_str("</turbo-stream>");
        out
    }
}

impl fmt::Display for TurboStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_html())
    }
}

/// Several `<turbo-stream>` elements sent together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurboStreams(pub Vec<TurboStream>);

impl TurboStreams {
    /// An empty list.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Add an element.
    #[must_use]
    pub fn push(mut self, stream: TurboStream) -> Self {
        self.0.push(stream);
        self
    }

    /// All elements as HTML, one after another.
    #[must_use]
    pub fn to_html(&self) -> String {
        self.0.iter().map(TurboStream::to_html).collect()
    }
}

impl From<Vec<TurboStream>> for TurboStreams {
    fn from(v: Vec<TurboStream>) -> Self {
        Self(v)
    }
}

impl From<TurboStream> for TurboStreams {
    fn from(v: TurboStream) -> Self {
        Self(vec![v])
    }
}

fn stream_response(body: String) -> Response {
    (
        [(header::CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE))],
        body,
    )
        .into_response()
}

impl IntoResponse for TurboStream {
    fn into_response(self) -> Response {
        stream_response(self.to_html())
    }
}

impl IntoResponse for TurboStreams {
    fn into_response(self) -> Response {
        stream_response(self.to_html())
    }
}

fn escape_attr(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

/// Whether the request's `Accept` header asks for a Turbo Stream answer.
///
/// Turbo adds the media type to `Accept` on form submissions, so this is the
/// check Rails' `format.turbo_stream` makes. An entry with `q=0` means "not
/// acceptable" and does not count.
#[must_use]
pub fn accepts_turbo_stream(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::ACCEPT)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .any(|part| {
            let mut params = part.split(';');
            params
                .next()
                .is_some_and(|m| m.trim().eq_ignore_ascii_case(CONTENT_TYPE))
                && !params.any(is_zero_quality)
        })
}

/// `true` for a `q=0` (or `q=0.0`, `q=0.000`) media-type parameter.
fn is_zero_quality(param: &str) -> bool {
    param.split_once('=').is_some_and(|(name, value)| {
        name.trim().eq_ignore_ascii_case("q") && value.trim().parse::<f32>().is_ok_and(|q| q <= 0.0)
    })
}

/// Extractor: `true` when the request accepts a Turbo Stream answer.
///
/// ```rust,ignore
/// async fn create(turbo: TurboStreamRequest, ...) -> Result<Response> {
///     if turbo.0 {
///         return Ok(TurboStream::prepend("posts", html).into_response());
///     }
///     format::redirect("/posts")
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurboStreamRequest(pub bool);

impl<S> FromRequestParts<S> for TurboStreamRequest
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    // Header inspection only — see `extractor/auth.rs` for why the `async fn`
    // shape stays anyway.
    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Infallible> {
        Ok(Self(accepts_turbo_stream(&parts.headers)))
    }
}

const DEFAULT_CAPACITY: usize = 64;

/// In-process hub that delivers Turbo Stream messages to subscribers of a
/// named stream (`broadcast_*_to` in Rails).
///
/// Cheap to clone; clones share the same streams. Put one in the shared store
/// in `Hooks::after_context` and read it back wherever there is an
/// `AppContext`:
///
/// ```rust
/// use loco_rs::{app::SharedStore, controller::turbo::{Broadcaster, TurboStream}};
///
/// let store = SharedStore::default();
/// store.insert(Broadcaster::default());
///
/// let hub = store.get::<Broadcaster>().unwrap();
/// let mut rx = hub.subscribe("posts");
/// assert_eq!(hub.broadcast("posts", TurboStream::remove("post_1")), 1);
/// assert_eq!(
///     rx.try_recv().unwrap(),
///     r#"<turbo-stream action="remove" target="post_1"></turbo-stream>"#
/// );
/// ```
///
/// Messages are not stored: a subscriber gets what is broadcast while it is
/// connected, and a subscriber that falls more than `capacity` messages behind
/// skips the oldest ones (an SSE subscriber is sent
/// [`TurboStream::refresh`] instead, so the page reloads its state). Only
/// subscribers connected to the same process receive a message.
///
/// The default [`crate::app::Hooks::serve`] calls [`Broadcaster::shutdown`]
/// on the shared store's broadcaster when the server stops, so open SSE
/// connections do not hold up a graceful shutdown. An app that overrides
/// `serve` must call it itself.
#[derive(Debug, Clone)]
pub struct Broadcaster {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    capacity: usize,
    keep_alive: Duration,
    streams: DashMap<String, broadcast::Sender<String>>,
    shutdown: CancellationToken,
}

/// An SSE subscription. Dropping it (the client went away, or the stream
/// ended) removes the stream's channel once nobody else listens, so per-record
/// streams that never see another broadcast do not pile up.
struct Subscription {
    rx: broadcast::Receiver<String>,
    inner: Arc<Inner>,
    stream: String,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        // `self.rx` is still alive here, so it counts as one receiver. The
        // check runs under the map's shard lock, which `subscribe` also
        // takes, so no new receiver can appear between the count and the
        // removal.
        self.inner
            .streams
            .remove_if(&self.stream, |_, tx| tx.receiver_count() <= 1);
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl Broadcaster {
    /// A hub whose subscribers buffer up to `capacity` messages each.
    ///
    /// # Panics
    ///
    /// When `capacity` is zero.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "broadcaster capacity must be greater than 0");
        Self {
            inner: Arc::new(Inner {
                capacity,
                keep_alive: Duration::from_secs(15),
                streams: DashMap::new(),
                shutdown: CancellationToken::new(),
            }),
        }
    }

    /// Subscribe to `stream`. The receiver yields each message as HTML.
    ///
    /// The stream's channel is dropped on the next [`Broadcaster::broadcast`]
    /// after the last receiver goes away. [`Broadcaster::sse`] cleans up as
    /// soon as the client disconnects.
    #[must_use]
    pub fn subscribe(&self, stream: &str) -> broadcast::Receiver<String> {
        self.inner
            .streams
            .entry(stream.to_string())
            .or_insert_with(|| broadcast::channel(self.inner.capacity).0)
            .subscribe()
    }

    /// Send `message` to everyone subscribed to `stream` and return how many
    /// subscribers it reached. Nobody listening is not an error: it returns 0.
    pub fn broadcast(&self, stream: &str, message: impl Into<TurboStreams>) -> usize {
        let html = message.into().to_html();
        let sent = self
            .inner
            .streams
            .get(stream)
            .and_then(|tx| tx.send(html).ok());
        if sent.is_none() {
            // Nobody left on this stream: drop the channel.
            self.inner
                .streams
                .remove_if(stream, |_, tx| tx.receiver_count() == 0);
        }
        sent.unwrap_or(0)
    }

    /// Number of subscribers currently connected to `stream`.
    #[must_use]
    pub fn subscriber_count(&self, stream: &str) -> usize {
        self.inner
            .streams
            .get(stream)
            .map_or(0, |tx| tx.receiver_count())
    }

    /// Subscribe to `stream` and answer with a Server-Sent Events response, for
    /// a page's `<turbo-stream-source src="...">`.
    ///
    /// Deciding who may read which stream is the handler's job: pick the
    /// stream name from the signed-in user or check access before calling
    /// this, as Rails does with signed stream names.
    pub fn sse(
        &self,
        stream: &str,
    ) -> Sse<impl Stream<Item = Result<Event, Infallible>> + Send + use<>> {
        let events = self
            .messages(stream)
            .map(|html| Ok(Event::default().data(html)));
        Sse::new(events).keep_alive(KeepAlive::new().interval(self.inner.keep_alive))
    }

    /// The HTML messages an SSE subscriber receives. Ends on
    /// [`Broadcaster::shutdown`]; a lagging subscriber gets a refresh.
    fn messages(&self, stream: &str) -> impl Stream<Item = String> + Send + use<> {
        let sub = Subscription {
            rx: self.subscribe(stream),
            inner: Arc::clone(&self.inner),
            stream: stream.to_string(),
        };
        stream::unfold(sub, |mut sub| async move {
            match sub.rx.recv().await {
                Ok(html) => Some((html, sub)),
                // Messages were skipped: have the page reload its state.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    Some((TurboStream::refresh().to_html(), sub))
                }
                Err(broadcast::error::RecvError::Closed) => None,
            }
        })
        .take_until(self.inner.shutdown.clone().cancelled_owned())
    }

    /// End every open [`Broadcaster::sse`] stream, now and from now on, so a
    /// graceful server shutdown is not held open by connected browsers.
    /// Turbo reconnects the page's stream source when the server is back.
    ///
    /// The default [`crate::app::Hooks::serve`] calls this after
    /// `on_shutdown`. Call it yourself if you override `serve`.
    pub fn shutdown(&self) {
        self.inner.shutdown.cancel();
    }

    /// Shut down the broadcaster in `store`, if there is one.
    pub(crate) fn shutdown_in(store: &crate::app::SharedStore) {
        if let Some(hub) = store.get_ref::<Self>() {
            hub.shutdown();
        }
    }

    /// Fetch the broadcaster from the shared store.
    ///
    /// # Errors
    ///
    /// When no [`Broadcaster`] was inserted into the shared store.
    pub fn from_store(store: &crate::app::SharedStore) -> Result<Self> {
        store.get::<Self>().ok_or_else(|| {
            Error::Message(
                "no Broadcaster in the shared store; insert one in Hooks::after_context".into(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_actions() {
        assert_eq!(
            TurboStream::append("posts", "<li>a</li>").to_html(),
            r#"<turbo-stream action="append" target="posts"><template><li>a</li></template></turbo-stream>"#
        );
        assert_eq!(
            TurboStream::remove("post_1").to_html(),
            r#"<turbo-stream action="remove" target="post_1"></turbo-stream>"#
        );
        assert_eq!(
            TurboStream::refresh().to_html(),
            r#"<turbo-stream action="refresh"></turbo-stream>"#
        );
        assert_eq!(
            TurboStream::new_all(Action::Update, ".count")
                .template("3")
                .to_html(),
            r#"<turbo-stream action="update" targets=".count"><template>3</template></turbo-stream>"#
        );
        for (s, a) in [
            (TurboStream::prepend("t", ""), "prepend"),
            (TurboStream::replace("t", ""), "replace"),
            (TurboStream::update("t", ""), "update"),
            (TurboStream::before("t", ""), "before"),
            (TurboStream::after("t", ""), "after"),
        ] {
            assert!(s
                .to_html()
                .starts_with(&format!(r#"<turbo-stream action="{a}""#)));
        }
    }

    #[test]
    fn renders_a_view_into_the_template() {
        struct Fake;
        impl ViewRenderer for Fake {
            fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
                Ok(format!("{key}:{}", serde_json::to_string(&data)?))
            }
        }
        let s = TurboStream::new(Action::Append, "posts")
            .render(&Fake, "posts/_post.html", serde_json::json!({"id": 1}))
            .unwrap();
        assert_eq!(
            s.to_html(),
            r#"<turbo-stream action="append" target="posts"><template>posts/_post.html:{"id":1}</template></turbo-stream>"#
        );
    }

    #[test]
    fn escapes_target() {
        assert_eq!(
            TurboStream::remove(r#"a"><script>"#).to_html(),
            r#"<turbo-stream action="remove" target="a&quot;&gt;&lt;script&gt;"></turbo-stream>"#
        );
    }

    #[test]
    fn joins_streams() {
        let s = TurboStreams::new()
            .push(TurboStream::remove("a"))
            .push(TurboStream::remove("b"));
        assert_eq!(
            s.to_html(),
            concat!(
                r#"<turbo-stream action="remove" target="a"></turbo-stream>"#,
                r#"<turbo-stream action="remove" target="b"></turbo-stream>"#
            )
        );
    }

    #[test]
    fn response_has_content_type() {
        let res = TurboStream::remove("a").into_response();
        assert_eq!(res.headers()[header::CONTENT_TYPE], CONTENT_TYPE);
    }

    #[test]
    fn detects_accept() {
        let mut h = HeaderMap::new();
        assert!(!accepts_turbo_stream(&h));
        h.insert(header::ACCEPT, HeaderValue::from_static("text/html"));
        assert!(!accepts_turbo_stream(&h));
        h.insert(
            header::ACCEPT,
            HeaderValue::from_static(
                "text/vnd.turbo-stream.html, text/html, application/xhtml+xml",
            ),
        );
        assert!(accepts_turbo_stream(&h));
        h.insert(
            header::ACCEPT,
            HeaderValue::from_static("text/html;q=0.9, TEXT/VND.TURBO-STREAM.HTML;q=1"),
        );
        assert!(accepts_turbo_stream(&h));
        for refused in [
            "text/vnd.turbo-stream.html;q=0, text/html",
            "text/vnd.turbo-stream.html; Q=0.0",
            "text/vnd.turbo-stream.html;level=1;q=0.000",
        ] {
            h.insert(header::ACCEPT, HeaderValue::from_static(refused));
            assert!(!accepts_turbo_stream(&h), "{refused}");
        }
        h.insert(
            header::ACCEPT,
            HeaderValue::from_static("text/vnd.turbo-stream.html;q=0.1"),
        );
        assert!(accepts_turbo_stream(&h));
    }

    #[test]
    fn broadcast_reaches_subscribers_of_that_stream_only() {
        let hub = Broadcaster::default();
        assert_eq!(hub.broadcast("posts", TurboStream::remove("x")), 0);

        let mut a = hub.subscribe("posts");
        let mut b = hub.subscribe("posts");
        let mut other = hub.subscribe("comments");
        assert_eq!(hub.subscriber_count("posts"), 2);

        assert_eq!(hub.broadcast("posts", TurboStream::remove("x")), 2);
        let expected = r#"<turbo-stream action="remove" target="x"></turbo-stream>"#;
        assert_eq!(a.try_recv().unwrap(), expected);
        assert_eq!(b.try_recv().unwrap(), expected);
        assert!(other.try_recv().is_err());
    }

    #[test]
    fn drops_streams_without_subscribers() {
        let hub = Broadcaster::default();
        drop(hub.subscribe("posts"));
        assert_eq!(hub.broadcast("posts", TurboStream::remove("x")), 0);
        assert!(hub.inner.streams.is_empty());
    }

    #[tokio::test]
    async fn sse_messages_end_on_shutdown() {
        let hub = Broadcaster::default();
        let mut open = Box::pin(hub.messages("posts"));
        hub.broadcast("posts", TurboStream::remove("a"));
        assert!(open.next().await.is_some());

        hub.clone().shutdown();
        let ended = tokio::time::timeout(Duration::from_secs(1), open.next()).await;
        assert_eq!(ended.expect("stream ends after shutdown"), None);

        // A stream opened after shutdown ends at once.
        let mut late = Box::pin(hub.messages("posts"));
        assert_eq!(late.next().await, None);
    }

    #[tokio::test]
    async fn shutdown_in_reaches_the_stored_broadcaster() {
        let store = crate::app::SharedStore::default();
        Broadcaster::shutdown_in(&store); // nothing stored: no panic
        let hub = Broadcaster::default();
        store.insert(hub.clone());
        let mut open = Box::pin(hub.messages("posts"));
        Broadcaster::shutdown_in(&store);
        assert_eq!(open.next().await, None);
    }

    #[test]
    fn dropping_an_sse_subscription_drops_its_channel() {
        let hub = Broadcaster::default();
        let first = hub.messages("post_1");
        let second = hub.messages("post_1");
        let raw = hub.subscribe("post_2");
        assert_eq!(hub.subscriber_count("post_1"), 2);

        drop(first);
        assert_eq!(hub.subscriber_count("post_1"), 1);
        assert!(hub.inner.streams.contains_key("post_1"));

        drop(second);
        assert!(!hub.inner.streams.contains_key("post_1"));
        // Plain receivers keep their channel until the next broadcast.
        drop(raw);
        assert_eq!(hub.inner.streams.len(), 1);
    }

    #[tokio::test]
    async fn lagging_sse_subscriber_gets_a_refresh() {
        let hub = Broadcaster::new(1);
        let mut events = Box::pin(hub.messages("posts"));
        for id in ["a", "b", "c"] {
            hub.broadcast("posts", TurboStream::remove(id));
        }
        assert_eq!(
            events.next().await.unwrap(),
            TurboStream::refresh().to_html()
        );
        assert_eq!(
            events.next().await.unwrap(),
            TurboStream::remove("c").to_html()
        );
    }

    #[test]
    fn clones_share_streams() {
        let hub = Broadcaster::default();
        let mut rx = hub.subscribe("posts");
        assert_eq!(hub.clone().broadcast("posts", TurboStream::refresh()), 1);
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn from_store() {
        let store = crate::app::SharedStore::default();
        assert!(Broadcaster::from_store(&store).is_err());
        store.insert(Broadcaster::default());
        assert!(Broadcaster::from_store(&store).is_ok());
    }
}
