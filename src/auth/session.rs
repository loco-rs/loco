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

use std::collections::BTreeMap;

use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts, OriginalUri},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, IntoResponseParts, Response, ResponseParts},
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use super::cookies::{self, CookieOptions, SignedCookies, DEFAULT_SESSION_COOKIE};
use crate::{app::AppContext, Error, Result};

/// The session key holding the signed-in user's id.
pub const USER_ID_KEY: &str = "_uid";

/// Browsers drop cookies over 4096 bytes (name, value and attributes); stay
/// well under so a full session is an error here, not a silent sign-out.
const MAX_COOKIE_VALUE: usize = 3800;

/// The cookie session. See the [module docs](self).
#[derive(Debug, Clone)]
pub struct Session {
    data: BTreeMap<String, Value>,
    jar: SignedCookies,
    name: String,
    max_age: u64,
    dirty: bool,
}

impl Session {
    /// Reads the session from a signed jar.
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
        }
    }

    /// The session cookie name configured for `ctx`.
    #[must_use]
    pub fn cookie_name(ctx: &AppContext) -> String {
        ctx.config
            .auth
            .as_ref()
            .and_then(|a| a.session.as_ref())
            .and_then(|s| s.cookie_name.clone())
            .unwrap_or_else(|| DEFAULT_SESSION_COOKIE.to_string())
    }

    /// The session for a request, outside an extractor.
    ///
    /// # Errors
    ///
    /// When the cookie key is unavailable (see [`cookies::key`]).
    pub fn from_parts(parts: &Parts, ctx: &AppContext) -> Result<Self> {
        let jar = SignedCookies::from_headers(
            &parts.headers,
            cookies::key(ctx)?,
            CookieOptions::from_config(&ctx.config),
        );
        Ok(Self::from_jar(jar, Self::cookie_name(ctx)))
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
    /// When `value` does not serialize, or the session would no longer fit in
    /// a cookie.
    pub fn insert<T: Serialize>(mut self, key: impl Into<String>, value: &T) -> Result<Self> {
        self.data.insert(key.into(), serde_json::to_value(value)?);
        let size = cookies::seal(&self.data, self.max_age)?.len();
        if size > MAX_COOKIE_VALUE {
            return Err(Error::Message(format!(
                "session is {size} bytes encoded, over the {MAX_COOKIE_VALUE} that fit in a \
                 cookie; store an id and keep the data server-side"
            )));
        }
        self.dirty = true;
        Ok(self)
    }

    /// Removes `key`.
    #[must_use]
    pub fn remove(mut self, key: &str) -> Self {
        self.dirty |= self.data.remove(key).is_some();
        self
    }

    /// Empties the session and deletes its cookie.
    #[must_use]
    pub fn clear(mut self) -> Self {
        self.data.clear();
        self.dirty = true;
        self
    }

    /// Empties the session, keeping the cookie, so a sign-in starts from a
    /// fresh session rather than one an attacker may have planted (session
    /// fixation). Call before [`set_user_id`](Self::set_user_id).
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
        self.dirty = true;
        self
    }

    /// Whether the session holds no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Hands back the signed jar, with this session's changes written to it.
    ///
    /// # Errors
    ///
    /// When the session does not serialize.
    pub fn into_jar(self) -> Result<SignedCookies> {
        if !self.dirty {
            return Ok(self.jar);
        }
        if self.data.is_empty() {
            return Ok(self.jar.remove(&self.name));
        }
        let value = cookies::seal(&self.data, self.max_age)?;
        let cookie = self.jar.options().cookie(self.name.clone(), value);
        Ok(self.jar.add(cookie))
    }
}

impl<S> FromRequestParts<S> for Session
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Self::from_parts(parts, &AppContext::from_ref(state))
    }
}

impl IntoResponseParts for Session {
    type Error = Response;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        match self.into_jar() {
            Ok(jar) => jar.into_response_parts(res).map_err(|never| match never {}),
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
    let session = Session::from_parts(parts, ctx)?;
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
