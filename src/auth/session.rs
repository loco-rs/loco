//! A cookie session and `CurrentUser` extractors — Rails' `session` plus
//! Devise's `current_user`.
//!
//! The session is a small JSON map stored in one signed cookie
//! (`auth.session.cookie_name`, default `loco_session`). It is tamper-evident
//! but **readable** by the browser, so keep it to identifiers; it carries its
//! own expiry (`auth.session.max_age`, default 30 days) that the server
//! enforces regardless of what the browser does with `Max-Age`.
//!
//! ```rust
//! use loco_rs::prelude::*;
//!
//! async fn sign_in(session: Session) -> Result<impl IntoResponse> {
//!     let session = session.renew().set_user_id("42");
//!     Ok((session, format::redirect("/")?))
//! }
//!
//! async fn dashboard(CurrentUser(user_id): CurrentUser<String>) -> Result<Response> {
//!     format::text(&format!("hello {user_id}"))
//! }
//!
//! async fn sign_out(session: Session) -> Result<impl IntoResponse> {
//!     Ok((session.clear(), format::redirect("/")?))
//! }
//! ```
//!
//! Return the [`Session`] from the handler (it is [`IntoResponseParts`]) for
//! changes to reach the browser; an unchanged session writes nothing.
//!
//! # Stores
//!
//! `auth.session.store` picks where the data lives. `cookie` (the default) is
//! what is described above. `database` and `redis` keep the data on the
//! server and the cookie carries only a signed random id, so a session can be
//! ended ([`Session::clear`]) or every session of a user can be
//! ([`session_store::sign_out_everywhere`]). The code above runs unchanged on
//! any store. Server-side stores need the `auth_session` middleware, which
//! [`default_middleware_stack`](crate::controller::middleware::default_middleware_stack)
//! includes.
//!
//! On a server-side store the session lives `auth.session.ttl` (default one
//! day, sliding) and its cookie ends with the browser. [`Session::remember`]
//! on a sign-in sets a second, long-lived cookie (`auth.session.remember_for`,
//! default 14 days) that brings the session back after that, and rotates on
//! every use. Server-side stores only.
//!
//! Setting a different user id ([`set_user_id`](Session::set_user_id)) mints a
//! new session id by itself, so a session id planted before sign-in is worth
//! nothing after it.

use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts, OriginalUri},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, IntoResponseParts, Response, ResponseParts},
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use super::{
    cookies::{self, CookieOptions, SignedCookies, DEFAULT_SESSION_COOKIE},
    session_store::{self, Op, Restored, SessionStore},
};
use crate::{app::AppContext, Error, Result};

/// The session key holding the signed-in user's id.
pub const USER_ID_KEY: &str = "_uid";

/// A server-side session cookie carries a signed expiry too; the server's
/// `ttl` is what ends the session, so this is just "very long".
const SERVER_SEAL_TTL: u64 = 60 * 60 * 24 * 365 * 10;

/// Browsers drop cookies over 4096 bytes (name, value and attributes); stay
/// well under so a full session is an error here, not a silent sign-out.
const MAX_COOKIE_VALUE: usize = 3800;

/// The session. See the [module docs](self).
///
/// What `auth.session.store` selects is invisible here: the same calls work on
/// the `cookie`, `database` and `redis` stores. On a server-side store the
/// changes are written by the `auth_session` middleware (installed by the
/// default middleware stack) when the response leaves the app.
#[derive(Clone)]
pub struct Session {
    data: BTreeMap<String, Value>,
    jar: SignedCookies,
    name: String,
    max_age: u64,
    dirty: bool,
    /// Sign-in asked to be remembered ([`remember`](Self::remember)).
    remember: bool,
    server: Option<Server>,
}

/// What a server-side session knows beyond the data.
#[derive(Clone)]
struct Server {
    store: Arc<SessionStore>,
    /// The live session's id.
    id: Option<String>,
    /// Mint a new id on save (session fixation defence).
    rotate: bool,
    /// [`Session::clear`] was called: the sign-in ended, whatever is written
    /// afterwards. Sticky.
    cleared: bool,
    /// The user id the session was loaded with; a different one rotates the id.
    loaded_uid: Option<Value>,
    /// The remember-me series this browser presented.
    series: Option<String>,
    remember_name: String,
    /// The session was re-established from a remember-me cookie.
    restored: bool,
    /// Less than half the ttl is left: extend it even if nothing changed.
    refresh: bool,
}

// Never prints ids, tokens or values: logs are not a place for them.
impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("keys", &self.data.keys().collect::<Vec<_>>())
            .field("store", &self.server.as_ref().map(|s| s.store.kind()))
            .field("dirty", &self.dirty)
            .finish_non_exhaustive()
    }
}

/// Session writes for the middleware to apply before the response is sent.
#[derive(Clone)]
pub(crate) struct Pending {
    pub store: Arc<SessionStore>,
    pub ops: Vec<Op>,
}

/// Marks a response a [`Session`] was written to.
#[derive(Clone)]
pub(crate) struct Handled;

/// Per-request state the `auth_session` middleware installs: the session is
/// loaded once per request and shared by every extractor.
#[derive(Default)]
pub(crate) struct Runtime {
    loaded: Mutex<Option<Session>>,
}

impl Runtime {
    fn cached(&self) -> Option<Session> {
        self.loaded.lock().ok()?.clone()
    }

    fn put(&self, session: &Session) {
        if let Ok(mut slot) = self.loaded.lock() {
            *slot = Some(session.clone());
        }
    }

    /// The session, when this request re-established it from a remember-me
    /// cookie (whose cookies still have to reach the browser).
    pub(crate) fn restored(&self) -> Option<Session> {
        self.cached()
            .filter(|s| s.server.as_ref().is_some_and(|s| s.restored))
    }

    /// The write that slides the expiry of a session this request only read.
    pub(crate) fn refresh(&self) -> Option<(Arc<SessionStore>, Op)> {
        let session = self.cached()?;
        let server = session.server.as_ref()?;
        let id = server.id.clone().filter(|_| server.refresh)?;
        let record = server.store.record(session.data.clone());
        Some((server.store.clone(), Op::Update { id, record }))
    }
}

fn session_config(ctx: &AppContext) -> Option<&crate::config::SessionConfig> {
    ctx.config.auth.as_ref().and_then(|a| a.session.as_ref())
}

impl Session {
    /// Reads a cookie-store session from a signed jar.
    #[must_use]
    pub fn from_jar(jar: SignedCookies, name: impl Into<String>) -> Self {
        let name = name.into();
        let max_age = jar
            .options()
            .max_age
            .unwrap_or(cookies::DEFAULT_SESSION_MAX_AGE);
        let data = jar
            .get(&name)
            .and_then(|c| cookies::unseal::<BTreeMap<String, Value>>(c.value()))
            .unwrap_or_default();
        Self {
            data,
            jar,
            name,
            max_age,
            dirty: false,
            remember: false,
            server: None,
        }
    }

    /// The session cookie name configured for `ctx`.
    #[must_use]
    pub fn cookie_name(ctx: &AppContext) -> String {
        session_config(ctx)
            .and_then(|s| s.cookie_name.clone())
            .unwrap_or_else(|| DEFAULT_SESSION_COOKIE.to_string())
    }

    /// The remember-me cookie name configured for `ctx`.
    #[must_use]
    pub fn remember_cookie_name(ctx: &AppContext) -> String {
        session_config(ctx)
            .and_then(|s| s.remember_cookie_name.clone())
            .unwrap_or_else(|| session_store::DEFAULT_REMEMBER_COOKIE.to_string())
    }

    /// The session for a request, outside an extractor.
    ///
    /// On a server-side store this reads the store (once per request), and may
    /// re-establish the session from a remember-me cookie.
    ///
    /// # Errors
    ///
    /// When the cookie key is unavailable (see [`cookies::key`]), or the
    /// server-side store cannot be reached or the `auth_session` middleware is
    /// not installed.
    pub async fn from_parts(parts: &Parts, ctx: &AppContext) -> Result<Self> {
        let jar = SignedCookies::from_headers(
            &parts.headers,
            cookies::key(ctx)?,
            CookieOptions::from_config(&ctx.config),
        );
        let name = Self::cookie_name(ctx);
        if !session_config(ctx).is_some_and(|s| s.store.is_server_side()) {
            return Ok(Self::from_jar(jar, name));
        }
        let runtime = parts.extensions.get::<Arc<Runtime>>().ok_or_else(|| {
            Error::Message(
                "auth.session.store is server-side, but the `auth_session` middleware is not \
                 installed. It is part of the default middleware stack; if `Hooks::middlewares` \
                 builds its own stack, add `middleware::auth_session::AuthSession`."
                    .to_string(),
            )
        })?;
        if let Some(session) = runtime.cached() {
            return Ok(session);
        }
        let session = Self::load_server(ctx, jar, name).await?;
        runtime.put(&session);
        Ok(session)
    }

    async fn load_server(ctx: &AppContext, jar: SignedCookies, name: String) -> Result<Self> {
        let store = session_store::of(ctx).await?;
        let options = jar.options().clone().with_max_age(None);
        let max_age = options.max_age.unwrap_or(cookies::DEFAULT_SESSION_MAX_AGE);
        let remember_name = Self::remember_cookie_name(ctx);
        let mut jar = jar;

        let mut id = None;
        let mut data = BTreeMap::new();
        let mut refresh = false;
        if let Some(presented) = jar
            .get(&name)
            .and_then(|c| cookies::unseal::<String>(c.value()))
            && let Some(record) = store.load(&presented).await?
        {
            refresh =
                record.expires_at - session_store::now() < session_store::to_i64(store.ttl()) / 2;
            data = record.data;
            id = Some(presented);
        }

        let presented = jar
            .get(&remember_name)
            .and_then(|c| cookies::unseal::<String>(c.value()))
            .and_then(|v| {
                v.split_once('.')
                    .map(|(series, token)| (series.to_string(), token.to_string()))
            });
        let mut series = None;
        let mut restored = false;
        if let Some((presented_series, token)) = presented {
            if id.is_some() {
                series = Some(presented_series);
            } else {
                match store.restore(&presented_series, &token).await? {
                    Restored::Ok { user, token } => {
                        let new_id = session_store::random_token();
                        data = BTreeMap::from([(USER_ID_KEY.to_string(), Value::String(user))]);
                        store
                            .apply(vec![Op::Insert {
                                id: new_id.clone(),
                                record: store.record(data.clone()),
                            }])
                            .await?;
                        jar = jar.add(
                            options.cookie(name.clone(), cookies::seal(&new_id, SERVER_SEAL_TTL)?),
                        );
                        if let Some(token) = token {
                            let value = format!("{presented_series}.{token}");
                            jar = jar.add(
                                options
                                    .clone()
                                    .with_max_age(Some(store.remember_for()))
                                    .cookie(
                                        remember_name.clone(),
                                        cookies::seal(&value, store.remember_for())?,
                                    ),
                            );
                        }
                        id = Some(new_id);
                        series = Some(presented_series);
                        restored = true;
                    }
                    Restored::Rejected => jar = jar.remove(&remember_name),
                }
            }
        }

        let loaded_uid = data.get(USER_ID_KEY).cloned();
        Ok(Self {
            data,
            jar,
            name,
            max_age,
            dirty: false,
            remember: false,
            server: Some(Server {
                store,
                id,
                rotate: false,
                cleared: false,
                loaded_uid,
                series,
                remember_name,
                restored,
                refresh,
            }),
        })
    }

    /// The value at `key`, if present and of type `T`.
    #[must_use]
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Stores `value` at `key`.
    ///
    /// # Errors
    ///
    /// When `value` does not serialize, or (cookie store) the session would no
    /// longer fit in a cookie.
    pub fn insert<T: Serialize>(mut self, key: impl Into<String>, value: &T) -> Result<Self> {
        self.data.insert(key.into(), serde_json::to_value(value)?);
        if self.server.is_none() {
            let size = cookies::seal(&self.data, self.max_age)?.len();
            if size > MAX_COOKIE_VALUE {
                return Err(Error::Message(format!(
                    "session is {size} bytes encoded, over the {MAX_COOKIE_VALUE} that fit in a \
                     cookie; store an id and keep the data server-side"
                )));
            }
        }
        self.touch();
        Ok(self)
    }

    /// Removes `key`.
    #[must_use]
    pub fn remove(mut self, key: &str) -> Self {
        self.dirty |= self.data.remove(key).is_some();
        self
    }

    /// Ends the session: empties it and deletes its cookie. On a server-side
    /// store the stored session, and this browser's remember-me token, are
    /// deleted too.
    #[must_use]
    pub fn clear(mut self) -> Self {
        self.data.clear();
        self.dirty = true;
        if let Some(server) = &mut self.server {
            server.cleared = true;
            server.rotate = true;
        }
        self
    }

    /// Empties the session so a sign-in starts from a fresh one rather than
    /// one an attacker may have planted (session fixation). Call before
    /// [`set_user_id`](Self::set_user_id). On a server-side store the next
    /// save also issues a new session id and deletes the old one.
    #[must_use]
    pub fn renew(self) -> Self {
        self.clear()
    }

    /// The signed-in user's id.
    #[must_use]
    pub fn user_id(&self) -> Option<String> {
        self.get(USER_ID_KEY)
    }

    /// Signs `id` in.
    #[must_use]
    pub fn set_user_id(mut self, id: impl Into<String>) -> Self {
        self.data
            .insert(USER_ID_KEY.to_string(), Value::String(id.into()));
        self.touch();
        self
    }

    /// Remember this sign-in across browser restarts, Devise's `remember_me`:
    /// a long-lived cookie (`auth.session.remember_for`, default 14 days)
    /// signs the user back in once the session itself has expired. Call it
    /// together with [`set_user_id`](Self::set_user_id), only when the user
    /// asked for it.
    ///
    /// Needs a server-side store.
    ///
    /// # Errors
    ///
    /// With `auth.session.store: cookie`, which cannot hold the token's
    /// server-side half. Decide before sending a response, not after.
    pub fn remember(mut self) -> Result<Self> {
        if self.server.is_none() {
            return Err(Error::Message(
                "Session::remember needs a server-side session store: set auth.session.store to \
                 `database` or `redis`"
                    .to_string(),
            ));
        }
        self.remember = true;
        self.touch();
        Ok(self)
    }

    /// Whether the session holds no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn touch(&mut self) {
        self.dirty = true;
    }

    /// Hands back the signed jar, with this session's changes written to it.
    ///
    /// # Errors
    ///
    /// When the session does not serialize, or it lives in a server-side
    /// store, where changes also need the store: return the [`Session`] from
    /// the handler instead.
    pub fn into_jar(self) -> Result<SignedCookies> {
        let (jar, pending) = self.finalize()?;
        if pending.is_some() {
            return Err(Error::Message(
                "a server-side session has to be returned from the handler, not turned into a \
                 jar"
                .to_string(),
            ));
        }
        Ok(jar)
    }

    /// The cookies to send and the store writes to make.
    fn finalize(self) -> Result<(SignedCookies, Option<Pending>)> {
        let Self {
            data,
            jar,
            name,
            max_age,
            dirty,
            remember,
            server,
        } = self;
        let Some(server) = server else {
            if !dirty {
                return Ok((jar, None));
            }
            if data.is_empty() {
                return Ok((jar.remove(&name), None));
            }
            let value = cookies::seal(&data, max_age)?;
            let cookie = jar.options().cookie(name, value);
            return Ok((jar.add(cookie), None));
        };
        if !dirty {
            return Ok((jar, None));
        }

        let store = server.store.clone();
        let mut jar = jar;
        // Remember-me removals go first, then the session, then additions: a
        // failure part-way never leaves a live remember series behind a
        // session that was meant to end.
        let mut before = Vec::new();
        let mut after = Vec::new();

        if data.is_empty() {
            if let Some(id) = server.id {
                after.push(Op::Delete { id });
            }
            jar = jar.remove(&name);
            if let Some(series) = server.series {
                before.push(Op::RememberDelete { series });
                jar = jar.remove(&server.remember_name);
            }
            before.extend(after);
            return Ok((jar, Some(Pending { store, ops: before })));
        }

        // A different user id gets a new session id even if the app forgot
        // `renew()`: whoever planted the old id must not inherit the sign-in.
        let changed_user =
            server.id.is_some() && data.get(USER_ID_KEY) != server.loaded_uid.as_ref();
        let rotating = server.rotate || changed_user;

        // The remember series belongs to the sign-in that asked for it. When
        // the id rotates or the session was cleared, that sign-in is over.
        let mut series = server.series;
        if (rotating || server.cleared || remember)
            && let Some(old) = series.take()
        {
            before.push(Op::RememberDelete { series: old });
            if !remember {
                jar = jar.remove(&server.remember_name);
            }
        }

        let mut ops = Vec::new();
        let (id, fresh) = match server.id {
            Some(id) if !rotating => (id, false),
            old => {
                if let Some(old) = old {
                    ops.push(Op::Delete { id: old });
                }
                (session_store::random_token(), true)
            }
        };
        let record = store.record(data);
        let user = record.user.clone();
        ops.push(if fresh {
            Op::Insert {
                id: id.clone(),
                record,
            }
        } else {
            Op::Update {
                id: id.clone(),
                record,
            }
        });
        let cookie = jar
            .options()
            .clone()
            .with_max_age(None)
            .cookie(name, cookies::seal(&id, SERVER_SEAL_TTL)?);
        jar = jar.add(cookie);

        if remember {
            let user = user.ok_or_else(|| {
                Error::Message("Session::remember needs a signed-in user".to_string())
            })?;
            let (put, value) = store.new_remember(&user);
            after.push(put);
            let cookie = jar
                .options()
                .clone()
                .with_max_age(Some(store.remember_for()))
                .cookie(
                    server.remember_name,
                    cookies::seal(&value, store.remember_for())?,
                );
            jar = jar.add(cookie);
        }
        before.extend(ops);
        before.extend(after);
        Ok((jar, Some(Pending { store, ops: before })))
    }

    /// Applies a change to the jar this session writes (e.g. removing a
    /// cookie of your own in the same response).
    #[must_use]
    pub fn map_jar(mut self, f: impl FnOnce(SignedCookies) -> SignedCookies) -> Self {
        self.jar = f(self.jar);
        self
    }
}

impl<S> FromRequestParts<S> for Session
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Self::from_parts(parts, &AppContext::from_ref(state)).await
    }
}

impl IntoResponseParts for Session {
    type Error = Response;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        let server_side = self.server.is_some();
        match self.finalize() {
            Ok((jar, pending)) => {
                let mut res = jar
                    .into_response_parts(res)
                    .map_err(|never| match never {})?;
                if server_side {
                    res.extensions_mut().insert(Handled);
                }
                if let Some(mut pending) = pending {
                    if let Some(earlier) = res.extensions_mut().remove::<Pending>() {
                        pending.ops.splice(0..0, earlier.ops);
                    }
                    res.extensions_mut().insert(pending);
                }
                Ok(res)
            }
            Err(err) => Err(err.into_response()),
        }
    }
}

/// How [`CurrentUser`] loads the signed-in user from the id in the session.
///
/// ```rust,ignore
/// use loco_rs::{auth::session::SessionUser, prelude::*};
/// use crate::models::_entities::users;
///
/// #[async_trait]
/// impl SessionUser for users::Model {
///     async fn find_by_session_id(ctx: &AppContext, id: &str) -> Result<Option<Self>> {
///         Ok(users::Entity::find()
///             .filter(users::Column::Pid.eq(id))
///             .one(&ctx.db)
///             .await?)
///     }
/// }
/// ```
///
/// Returning `Ok(None)` (a deleted user, a stale id) treats the request as
/// signed out. Use a stable, non-guessable id such as a `pid`, not a
/// sequential primary key, so a leaked secret is the only way to forge one.
#[async_trait::async_trait]
pub trait SessionUser: Clone + Send + Sync + Sized + 'static {
    /// Loads the user whose session id is `id`.
    async fn find_by_session_id(ctx: &AppContext, id: &str) -> Result<Option<Self>>;
}

/// The id itself — for apps that need only "who", not a loaded model.
#[async_trait::async_trait]
impl SessionUser for String {
    async fn find_by_session_id(_ctx: &AppContext, id: &str) -> Result<Option<Self>> {
        Ok(Some(id.to_string()))
    }
}

/// The signed-in user; rejects signed-out requests.
///
/// A browser request (one that accepts `text/html`) is redirected to
/// `auth.session.sign_in_path?return_to=<path>` when that is configured;
/// anything else gets `401`. Take [`MaybeCurrentUser`] or
/// `Option<CurrentUser<U>>` for pages that serve both.
#[derive(Debug, Clone)]
pub struct CurrentUser<U>(pub U);

/// The signed-in user, or `None`. Never rejects for being signed out.
#[derive(Debug, Clone)]
pub struct MaybeCurrentUser<U>(pub Option<U>);

/// Request-scoped cache so several extractors share one lookup.
#[derive(Clone)]
struct Loaded<U>(Option<U>);

async fn load<U: SessionUser>(parts: &mut Parts, ctx: &AppContext) -> Result<Option<U>> {
    if let Some(Loaded(user)) = parts.extensions.get::<Loaded<U>>() {
        return Ok(user.clone());
    }
    let session = Session::from_parts(parts, ctx).await?;
    let user = match session.user_id() {
        Some(id) => U::find_by_session_id(ctx, &id).await?,
        None => None,
    };
    parts.extensions.insert(Loaded(user.clone()));
    Ok(user)
}

fn sign_in_required(parts: &Parts, ctx: &AppContext) -> Response {
    let sign_in_path = ctx
        .config
        .auth
        .as_ref()
        .and_then(|a| a.session.as_ref())
        .and_then(|s| s.sign_in_path.as_deref());
    let wants_html = parts
        .headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/html"));
    match sign_in_path {
        Some(path) if wants_html => {
            let uri = parts
                .extensions
                .get::<OriginalUri>()
                .map_or(&parts.uri, |o| &o.0);
            let here = uri.path_and_query().map_or("/", |p| p.as_str());
            let sep = if path.contains('?') { '&' } else { '?' };
            let location = format!("{path}{sep}return_to={}", encode(here));
            (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
        }
        _ => Error::Unauthorized("sign in required".to_string()).into_response(),
    }
}

/// Percent-encodes `value` for a query string.
#[must_use]
pub fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

impl<S, U> FromRequestParts<S> for CurrentUser<U>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
    U: SessionUser,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        match load::<U>(parts, &ctx).await {
            Ok(Some(user)) => Ok(Self(user)),
            Ok(None) => Err(sign_in_required(parts, &ctx)),
            Err(err) => Err(err.into_response()),
        }
    }
}

impl<S, U> OptionalFromRequestParts<S> for CurrentUser<U>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
    U: SessionUser,
{
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        load::<U>(parts, &ctx)
            .await
            .map(|u| u.map(Self))
            .map_err(IntoResponse::into_response)
    }
}

impl<S, U> FromRequestParts<S> for MaybeCurrentUser<U>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
    U: SessionUser,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        load::<U>(parts, &ctx)
            .await
            .map(Self)
            .map_err(IntoResponse::into_response)
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{HeaderMap, HeaderValue, Request},
    };

    use super::*;
    use crate::auth::cookies::{derive_key, tests::echo_cookies};

    fn opts() -> CookieOptions {
        CookieOptions {
            secure: false,
            max_age: Some(3600),
        }
    }

    fn session(headers: &HeaderMap) -> Session {
        Session::from_jar(
            SignedCookies::from_headers(headers, derive_key("session-test-secret"), opts()),
            "s",
        )
    }

    fn respond(session: Session) -> Response {
        (session, "ok").into_response()
    }

    #[test]
    fn values_survive_a_round_trip() {
        let res = respond(
            session(&HeaderMap::new())
                .insert("cart", &vec![1, 2])
                .unwrap()
                .set_user_id("u-1"),
        );
        let back = session(&echo_cookies(&res));
        assert_eq!(back.get::<Vec<i32>>("cart"), Some(vec![1, 2]));
        assert_eq!(back.user_id().as_deref(), Some("u-1"));
        assert_eq!(back.get::<String>("cart"), None, "wrong type reads as None");
    }

    #[test]
    fn an_unchanged_session_sets_no_cookie() {
        let res = respond(session(&HeaderMap::new()));
        assert!(res.headers().get(header::SET_COOKIE).is_none());
    }

    #[test]
    fn clear_deletes_the_cookie() {
        let res = respond(session(&HeaderMap::new()).set_user_id("u-1"));
        let res = respond(session(&echo_cookies(&res)).clear());
        let set = res
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(set.starts_with("s=;"), "{set}");
        assert!(set.contains("Max-Age=0"), "{set}");
    }

    #[test]
    fn renew_drops_planted_values() {
        let res = respond(session(&HeaderMap::new()).insert("planted", &1).unwrap());
        let renewed = session(&echo_cookies(&res)).renew().set_user_id("u-1");
        assert_eq!(renewed.get::<i32>("planted"), None);
        assert_eq!(renewed.user_id().as_deref(), Some("u-1"));
    }

    #[test]
    fn a_forged_session_reads_as_empty() {
        let forged = cookies::seal(
            &BTreeMap::from([(USER_ID_KEY.to_string(), Value::from("admin"))]),
            60,
        )
        .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("s={forged}")).unwrap(),
        );
        assert!(session(&headers).user_id().is_none());
    }

    #[test]
    fn an_oversized_session_is_an_error() {
        let big = "x".repeat(5000);
        assert!(session(&HeaderMap::new()).insert("big", &big).is_err());
    }

    fn parts(accept: &str, uri: &str) -> Parts {
        Request::builder()
            .uri(uri)
            .header(header::ACCEPT, accept)
            .body(Body::empty())
            .unwrap()
            .into_parts()
            .0
    }

    #[tokio::test]
    async fn signed_out_browsers_are_sent_to_sign_in_with_return_to() {
        let mut ctx = crate::tests_cfg::app::get_app_context().await;
        ctx.config.auth = Some(crate::config::Auth {
            session: Some(crate::config::SessionConfig {
                sign_in_path: Some("/login".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        });

        let mut p = parts("text/html,*/*", "/orgs/acme?tab=1");
        let rejection =
            <CurrentUser<String> as FromRequestParts<AppContext>>::from_request_parts(&mut p, &ctx)
                .await
                .unwrap_err();
        assert_eq!(rejection.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            rejection.headers()[header::LOCATION],
            "/login?return_to=%2Forgs%2Facme%3Ftab%3D1"
        );

        let mut p = parts("application/json", "/api/me");
        let rejection =
            <CurrentUser<String> as FromRequestParts<AppContext>>::from_request_parts(&mut p, &ctx)
                .await
                .unwrap_err();
        assert_eq!(rejection.status(), StatusCode::UNAUTHORIZED);

        let mut p = parts("text/html", "/");
        let MaybeCurrentUser(none) =
            <MaybeCurrentUser<String> as FromRequestParts<AppContext>>::from_request_parts(
                &mut p, &ctx,
            )
            .await
            .unwrap();
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn a_signed_in_request_yields_the_user() {
        let ctx = crate::tests_cfg::app::get_app_context().await;
        let jar = SignedCookies::from_headers(
            &HeaderMap::new(),
            cookies::key(&ctx).unwrap(),
            CookieOptions::from_config(&ctx.config),
        );
        let res = respond(Session::from_jar(jar, Session::cookie_name(&ctx)).set_user_id("u-9"));

        let mut p = parts("text/html", "/");
        p.headers = echo_cookies(&res);
        let CurrentUser(id) =
            <CurrentUser<String> as FromRequestParts<AppContext>>::from_request_parts(&mut p, &ctx)
                .await
                .unwrap();
        assert_eq!(id, "u-9");
    }
}
