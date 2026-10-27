//! `OAuth2` sign-in — Loco's `OmniAuth`.
//!
//! Configure providers under `auth.oauth.providers`, implement
//! [`OAuthHandler`] to turn an [`Identity`] into a user, and mount
//! [`routes`]:
//!
//! ```rust,ignore
//! use loco_rs::{auth::oauth::{self, Identity, OAuthHandler, SignIn}, prelude::*};
//!
//! pub struct SignInWithOAuth;
//!
//! #[async_trait]
//! impl OAuthHandler for SignInWithOAuth {
//!     async fn on_sign_in(ctx: &AppContext, identity: &Identity) -> Result<SignIn> {
//!         let user = users::Model::upsert_from_identity(&ctx.db, identity).await?;
//!         Ok(SignIn::new(user.pid.to_string()))
//!     }
//! }
//!
//! // in `App::routes`
//! AppRoutes::with_default_routes().add_route(oauth::routes::<SignInWithOAuth>())
//! ```
//!
//! That mounts:
//!
//! | route | does |
//! |---|---|
//! | `GET\|POST /auth/{provider}` | sets a state cookie, redirects to the provider. `?return_to=/path` (same-site paths only) is where the user lands afterwards. |
//! | `GET /auth/{provider}/callback` | checks state, exchanges the code, calls [`OAuthHandler::on_sign_in`], starts a fresh session. |
//! | `GET\|POST /auth/sign_out` | clears the session. |
//!
//! Read the user back with
//! [`CurrentUser`](crate::auth::session::CurrentUser). Routes of your own
//! under `/auth` (say `/auth/github/setup` for a GitHub App install callback)
//! coexist with these: a literal segment wins over `{provider}`.
//!
//! **Starting with POST.** A `GET` start lets any page sign a visitor in
//! with whatever account their browser is already logged in to at the
//! provider. That is harmless for plain sign-in but not for *account
//! linking* (`OmniAuth`'s CVE-2015-9284): an app that attaches the identity to
//! the already signed-in user should start the flow from a CSRF-protected
//! `POST` form.

pub mod provider;
pub mod state;

use axum::{
    extract::{OriginalUri, Path, Query, State},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;

pub use self::provider::Identity;
use self::{
    provider::Provider,
    state::{FlowState, STATE_TTL_SECONDS},
};
use super::{
    cookies::{self, SignedCookies},
    session::{encode, Session},
};
use crate::{
    app::AppContext,
    config::OAuthConfig,
    controller::{format, Routes},
    Error, Result,
};

/// What [`OAuthHandler::on_sign_in`] decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignIn {
    /// Stored in the session; what
    /// [`SessionUser::find_by_session_id`](crate::auth::session::SessionUser::find_by_session_id)
    /// receives.
    pub user_id: String,
    /// Where to go when the flow carried no `return_to` (e.g. `/onboarding`
    /// for a new account). Defaults to `auth.oauth.after_sign_in_path`, then
    /// `/`.
    pub redirect_to: Option<String>,
}

impl SignIn {
    /// Sign in the user with session id `user_id`.
    #[must_use]
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            redirect_to: None,
        }
    }

    /// Land on `path` unless the flow carried a `return_to`.
    #[must_use]
    pub fn redirect_to(mut self, path: impl Into<String>) -> Self {
        self.redirect_to = Some(path.into());
        self
    }
}

/// Why a sign-in did not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// The user declined at the provider (`error=access_denied`).
    Cancelled,
    /// The provider redirected back with another `error`.
    ProviderError,
    /// The state cookie was missing, expired or did not match: a stale tab,
    /// cookies blocked, or a forged callback.
    InvalidState,
    /// The callback had neither `code` nor `error`.
    MissingCode,
    /// The token endpoint refused the code or could not be reached.
    TokenExchange,
    /// The profile could not be fetched or had no account id.
    Profile,
    /// [`OAuthHandler::on_sign_in`] returned [`Error::Unauthorized`].
    Denied,
    /// The provider is unknown or misconfigured.
    Configuration,
}

impl FailureKind {
    /// A stable identifier (`cancelled`, `invalid_state`, …), used as
    /// `?error=` on `auth.oauth.failure_path`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::ProviderError => "provider_error",
            Self::InvalidState => "invalid_state",
            Self::MissingCode => "missing_code",
            Self::TokenExchange => "token_exchange",
            Self::Profile => "profile",
            Self::Denied => "denied",
            Self::Configuration => "configuration",
        }
    }

    /// A sentence fit to show the user.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Cancelled => "Sign-in was cancelled.",
            Self::InvalidState => "This sign-in link has expired. Please start again.",
            Self::MissingCode => "The provider did not complete the sign-in. Please try again.",
            Self::Denied => "This account is not allowed to sign in.",
            Self::ProviderError | Self::TokenExchange | Self::Profile | Self::Configuration => {
                "Sign-in failed. Please try again."
            }
        }
    }
}

/// A failed sign-in, handed to [`OAuthHandler::on_failure`].
#[derive(Debug, Clone)]
pub struct OAuthFailure {
    /// The provider's configured name.
    pub provider: String,
    /// What went wrong.
    pub kind: FailureKind,
    /// Diagnostic detail for logs. May contain provider- or
    /// attacker-supplied text: do not render it unescaped.
    pub detail: Option<String>,
    /// The path that starts this provider's flow again.
    pub retry_path: String,
}

/// The app side of the sign-in flow.
#[async_trait::async_trait]
pub trait OAuthHandler: Send + Sync + 'static {
    /// Find or create the user for `identity` and return their session id.
    ///
    /// Return [`Error::Unauthorized`] to refuse the sign-in (an account not
    /// on the allowlist, say); it becomes a [`FailureKind::Denied`] failure.
    /// Any other error is a `500`.
    async fn on_sign_in(ctx: &AppContext, identity: &Identity) -> Result<SignIn>;

    /// The response for a failed sign-in. Default: redirect to
    /// `auth.oauth.failure_path?error=<kind>&provider=<name>` when set,
    /// otherwise a minimal page with the message and a retry link.
    async fn on_failure(ctx: &AppContext, failure: &OAuthFailure) -> Result<Response> {
        default_failure(ctx, failure)
    }

    /// Where `/auth/sign_out` lands. Default
    /// `auth.oauth.after_sign_out_path`, then `/`.
    fn after_sign_out_path(ctx: &AppContext) -> String {
        oauth_config(ctx)
            .and_then(|c| c.after_sign_out_path.clone())
            .unwrap_or_else(|| "/".to_string())
    }
}

/// The routes for handler `H`, under `/auth`. Add your own with `.add(..)` or
/// in another controller.
#[must_use]
pub fn routes<H: OAuthHandler>() -> Routes {
    Routes::at("/auth")
        .add("/sign_out", get(sign_out::<H>).post(sign_out::<H>))
        .add("/{provider}", get(start).post(start))
        .add("/{provider}/callback", get(callback::<H>))
}

fn oauth_config(ctx: &AppContext) -> Option<&OAuthConfig> {
    ctx.config.auth.as_ref().and_then(|a| a.oauth.as_ref())
}

fn resolve(ctx: &AppContext, name: &str) -> Result<Provider> {
    let config = oauth_config(ctx)
        .and_then(|c| c.providers.get(name))
        .ok_or(Error::NotFound)?;
    Provider::resolve(name, config).map_err(Error::Message)
}

/// The public origin callbacks are built on.
fn base_url(ctx: &AppContext) -> String {
    oauth_config(ctx)
        .and_then(|c| c.base_url.clone())
        .unwrap_or_else(|| {
            let server = &ctx.config.server;
            // An https host is assumed to sit behind a proxy on 443.
            if server.host.starts_with("https://") {
                server.host.clone()
            } else {
                server.full_url()
            }
        })
        .trim_end_matches('/')
        .to_string()
}

fn redirect_uri(ctx: &AppContext, provider: &Provider, callback_path: &str) -> String {
    provider
        .redirect_uri
        .clone()
        .unwrap_or_else(|| format!("{}{callback_path}", base_url(ctx)))
}

/// Query of `/auth/{provider}`.
#[derive(Debug, Deserialize)]
pub struct StartParams {
    /// Where to land after sign-in. Same-site paths only.
    pub return_to: Option<String>,
}

/// Query of `/auth/{provider}/callback`.
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    /// Authorization code.
    pub code: Option<String>,
    /// Echoed state.
    pub state: Option<String>,
    /// Provider error code.
    pub error: Option<String>,
    /// Provider error description.
    pub error_description: Option<String>,
}

async fn start(
    State(ctx): State<AppContext>,
    Path(name): Path<String>,
    OriginalUri(uri): OriginalUri,
    Query(params): Query<StartParams>,
    jar: SignedCookies,
) -> Result<Response> {
    let provider = resolve(&ctx, &name)?;
    let flow = FlowState::new(provider.pkce, params.return_to.as_deref());
    let callback_path = format!("{}/callback", uri.path().trim_end_matches('/'));
    let location = provider
        .authorize_url(
            &redirect_uri(&ctx, &provider, &callback_path),
            &flow.state,
            flow.challenge().as_deref(),
        )
        .map_err(Error::Message)?;
    let cookie = jar
        .options()
        .clone()
        .with_max_age(Some(STATE_TTL_SECONDS))
        .cookie(
            state::cookie_name(&name),
            cookies::seal(&flow, STATE_TTL_SECONDS)?,
        );
    Ok((jar.add(cookie), format::redirect(&location)?).into_response())
}

async fn callback<H: OAuthHandler>(
    State(ctx): State<AppContext>,
    Path(name): Path<String>,
    OriginalUri(uri): OriginalUri,
    Query(params): Query<CallbackParams>,
    jar: SignedCookies,
) -> Result<Response> {
    let cookie_name = state::cookie_name(&name);
    let flow = jar
        .get(&cookie_name)
        .and_then(|c| cookies::unseal::<FlowState>(c.value()));
    // The state is single-use: clear it whatever happens next.
    let jar = jar.remove(&cookie_name);
    let retry_path = uri
        .path()
        .strip_suffix("/callback")
        .unwrap_or("/")
        .to_string();
    let fail = |kind: FailureKind, detail: Option<String>| OAuthFailure {
        provider: name.clone(),
        kind,
        detail,
        retry_path: retry_path.clone(),
    };

    let outcome = complete::<H>(&ctx, &name, uri.path(), params, flow, &fail).await;
    let response = match outcome {
        Ok((sign_in, return_to)) => {
            let target = return_to
                .or(sign_in.redirect_to)
                .or_else(|| oauth_config(&ctx).and_then(|c| c.after_sign_in_path.clone()))
                .unwrap_or_else(|| "/".to_string());
            let session = Session::from_jar(jar, Session::cookie_name(&ctx))
                .renew()
                .set_user_id(sign_in.user_id);
            return Ok((session, format::redirect(&target)?).into_response());
        }
        Err(Outcome::Failed(failure)) => {
            tracing::warn!(
                provider = %failure.provider,
                kind = failure.kind.as_str(),
                detail = failure.detail.as_deref().unwrap_or(""),
                "oauth sign-in failed"
            );
            H::on_failure(&ctx, &failure).await?
        }
        Err(Outcome::Error(err)) => return Err(err),
    };
    Ok((jar, response).into_response())
}

enum Outcome {
    Failed(OAuthFailure),
    Error(Error),
}

async fn complete<H: OAuthHandler>(
    ctx: &AppContext,
    name: &str,
    callback_path: &str,
    params: CallbackParams,
    flow: Option<FlowState>,
    fail: &(impl Fn(FailureKind, Option<String>) -> OAuthFailure + Sync),
) -> std::result::Result<(SignIn, Option<String>), Outcome> {
    let failed = |kind, detail| Outcome::Failed(fail(kind, detail));
    if let Some(error) = params.error {
        let kind = if error == "access_denied" {
            FailureKind::Cancelled
        } else {
            FailureKind::ProviderError
        };
        let detail = match params.error_description {
            Some(d) => format!("{error}: {d}"),
            None => error,
        };
        return Err(failed(kind, Some(detail)));
    }
    let Some(flow) = flow else {
        return Err(failed(
            FailureKind::InvalidState,
            Some("state cookie missing or expired".to_string()),
        ));
    };
    if !params.state.as_deref().is_some_and(|s| flow.matches(s)) {
        return Err(failed(
            FailureKind::InvalidState,
            Some("state mismatch".to_string()),
        ));
    }
    let Some(code) = params.code.filter(|c| !c.is_empty()) else {
        return Err(failed(FailureKind::MissingCode, None));
    };
    let provider =
        resolve(ctx, name).map_err(|e| failed(FailureKind::Configuration, Some(e.to_string())))?;
    let redirect_uri = redirect_uri(ctx, &provider, callback_path);
    let tokens = provider::exchange_code(&provider, &code, &redirect_uri, flow.verifier.as_deref())
        .await
        .map_err(|e| failed(FailureKind::TokenExchange, Some(e)))?;
    let identity = provider::fetch_identity(&provider, tokens)
        .await
        .map_err(|e| failed(FailureKind::Profile, Some(e)))?;
    match H::on_sign_in(ctx, &identity).await {
        Ok(sign_in) => Ok((sign_in, flow.return_to)),
        Err(Error::Unauthorized(reason)) => Err(failed(FailureKind::Denied, Some(reason))),
        Err(err) => Err(Outcome::Error(err)),
    }
}

async fn sign_out<H: OAuthHandler>(
    State(ctx): State<AppContext>,
    session: Session,
) -> Result<Response> {
    let target = H::after_sign_out_path(&ctx);
    Ok((session.clear(), format::redirect(&target)?).into_response())
}

/// [`OAuthHandler::on_failure`]'s default.
///
/// # Errors
///
/// When the redirect target is not a valid header value.
pub fn default_failure(ctx: &AppContext, failure: &OAuthFailure) -> Result<Response> {
    if let Some(path) = oauth_config(ctx).and_then(|c| c.failure_path.as_deref()) {
        let sep = if path.contains('?') { '&' } else { '?' };
        return format::redirect(&format!(
            "{path}{sep}error={}&provider={}",
            failure.kind.as_str(),
            encode(&failure.provider)
        ));
    }
    let page = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Sign-in failed</title>\
         </head><body><p>{}</p><p><a href=\"{}\">Try again</a></p></body></html>",
        escape(failure.kind.message()),
        escape(&failure.retry_path),
    );
    Ok((
        axum::http::StatusCode::UNAUTHORIZED,
        axum::response::Html(page),
    )
        .into_response())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_kinds_have_stable_ids() {
        let all = [
            FailureKind::Cancelled,
            FailureKind::ProviderError,
            FailureKind::InvalidState,
            FailureKind::MissingCode,
            FailureKind::TokenExchange,
            FailureKind::Profile,
            FailureKind::Denied,
            FailureKind::Configuration,
        ];
        let ids: Vec<_> = all.iter().map(|k| k.as_str()).collect();
        insta::assert_debug_snapshot!(ids);
    }

    #[test]
    fn escape_neutralizes_markup() {
        assert_eq!(
            escape("<a href=\"x\">'&'</a>"),
            "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;"
        );
    }
}
