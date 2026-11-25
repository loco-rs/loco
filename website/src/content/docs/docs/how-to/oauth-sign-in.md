---
title: Sign In with GitHub, Google or OAuth2
description: Add OAuth2 sign-in with the auth_oauth feature, keep users signed in with a signed cookie session, and read them back with CurrentUser.
sidebar:
  order: 44
---

Goal: let people sign in with GitHub, Google or any OAuth2/OpenID Connect provider, keep them signed in with a cookie, and know who they are in your handlers.

Three pieces ship together, in layers:

| Piece | Feature | Rails equivalent |
|---|---|---|
| `SignedCookies` (and `EncryptedCookies`) | `auth_session` (`auth_session_encrypted`) | `cookies.signed` / `cookies.encrypted` |
| `Session`, `CurrentUser<U>`, `MaybeCurrentUser<U>` | `auth_session` | `session`, Devise's `current_user` |
| `/auth/{provider}` routes, `OAuthHandler`, `Identity` | `auth_oauth` | OmniAuth |

`auth_oauth` turns on `auth_session`, which turns on `auth`. None of them is a default feature.

## The quick way: the generator

```sh
cargo loco generate oauth github
```

It writes `src/controllers/oauth.rs`, mounts its routes in `src/app.rs`, adds an `auth.session` secret and an `auth.oauth.providers.github` entry to `config/development.yaml`, `test.yaml` and `production.yaml`, and adds `auth_oauth` to `loco-rs`'s features in `Cargo.toml`. Run it again with another name (`google`, or `okta` for a generic provider) to add a second provider to the same controller.

Then register an OAuth app with the provider:

- **GitHub:** Settings → Developer settings → OAuth Apps. Authorization callback URL: `http://localhost:5150/auth/github/callback`.
- **Google:** Cloud Console → APIs & Services → Credentials → OAuth client ID (Web). Authorized redirect URI: `http://localhost:5150/auth/google/callback`.

Export the credentials and start the app:

```sh
export GITHUB_CLIENT_ID=... GITHUB_CLIENT_SECRET=...
cargo loco start
```

Visit `/auth/github`. The rest of this page is what the generator set up, and how to change it.

## 1. Enable the feature

```toml
loco-rs = { version = "...", features = ["auth_oauth"] }
```

`auth_oauth` adds `reqwest` (rustls, ring) for the server-to-server token and profile calls. `auth_session` adds nothing new to the build: signing uses the HMAC-SHA256 that `auth` already compiles. `auth_session_encrypted` adds `aes-gcm`.

## 2. Configure the session and providers

```yaml
auth:
  session:
    # Signs every cookie Loco sets. Required in production.
    secret: <%= get_env(name="SESSION_SECRET") %>
    # cookie_name: loco_session
    # max_age: 2592000          # seconds; 30 days
    # secure: true              # default: true when server.host is https://
    # sign_in_path: /login      # where CurrentUser sends signed-out browsers
    # return_to_param: return_to          # the query param it adds
    # navigational_formats: [html]        # which requests get the redirect
  oauth:
    # base_url: https://app.example.com   # public origin for callback URLs
    # after_sign_in_path: /
    # after_sign_out_path: /
    # failure_path: /login                # else a minimal error page
    # origin_param: return_to             # default: session.return_to_param
    # state_cookie_name: loco_oauth_state # prefix; the provider name is appended
    providers:
      github:
        client_id: <%= get_env(name="GITHUB_CLIENT_ID") %>
        client_secret: <%= get_env(name="GITHUB_CLIENT_SECRET") %>
        # scopes: [read:user, user:email]
      google:
        client_id: <%= get_env(name="GOOGLE_CLIENT_ID") %>
        client_secret: <%= get_env(name="GOOGLE_CLIENT_SECRET") %>
        authorize_params:
          prompt: select_account
      okta:
        strategy: generic
        client_id: <%= get_env(name="OKTA_CLIENT_ID") %>
        client_secret: <%= get_env(name="OKTA_CLIENT_SECRET") %>
        authorize_url: https://acme.okta.com/oauth2/v1/authorize
        token_url: https://acme.okta.com/oauth2/v1/token
        userinfo_url: https://acme.okta.com/oauth2/v1/userinfo
```

A provider named `github` or `google` uses that preset. Any other name needs `strategy: github | google | generic`. Every URL a preset fills in can be overridden:

| Key | Default |
|---|---|
| `authorize_url` | GitHub `{base_url}/login/oauth/authorize`, Google `https://accounts.google.com/o/oauth2/v2/auth` |
| `token_url` | GitHub `{base_url}/login/oauth/access_token`, Google `https://oauth2.googleapis.com/token` |
| `userinfo_url` | GitHub `{api_url}/user`, Google `https://openidconnect.googleapis.com/v1/userinfo` |
| `emails_url` | GitHub `{api_url}/user/emails` |
| `base_url` / `api_url` | GitHub only: `https://github.com` / `https://api.github.com`. Set `base_url` alone for GitHub Enterprise; `api_url` then defaults to `{base_url}/api/v3`. |
| `redirect_uri` | `{auth.oauth.base_url}{mount path}/{provider}/callback` |
| `scopes` | GitHub `[read:user, user:email]`, others `[openid, email, profile]` |
| `pkce` | `true` (S256). Providers without PKCE support ignore it. |
| `token_auth` | `post` (credentials in the form); `basic` for HTTP Basic |

Without `auth.oauth.base_url`, callback URLs are built from `server.host` (and `server.port`, unless the host is `https://`). Set `base_url` in production so the callback matches what you registered.

## 3. Handle the sign-in

Implement `OAuthHandler`. `on_sign_in` gets a normalized `Identity` and returns the id to keep in the session:

```rust
use loco_rs::{auth::oauth, prelude::*};

pub struct OAuth;

#[async_trait]
impl OAuthHandler for OAuth {
    async fn on_sign_in(ctx: &AppContext, identity: &Identity) -> Result<SignIn> {
        if identity.provider == "github" && identity.login.as_deref() == Some("blocked") {
            // Becomes a "denied" failure, not a 500.
            return Err(Error::Unauthorized("not allowed".to_string()));
        }
        let user_pid = upsert_user(ctx, identity).await?;
        Ok(SignIn::new(user_pid).redirect_to("/onboarding"))
    }
}

async fn upsert_user(_ctx: &AppContext, identity: &Identity) -> Result<String> {
    // Find or create the user keyed by (identity.provider, identity.uid).
    Ok(format!("{}:{}", identity.provider, identity.uid))
}

pub fn routes() -> Routes {
    oauth::routes::<OAuth>()
}
```

Mount it in `App::routes`:

```rust
AppRoutes::with_default_routes()
    .add_route(controllers::oauth::routes())
```

`Identity` carries `provider`, `uid`, `login`, `name`, `email`, `email_verified`, `avatar_url`, `access_token`, `refresh_token`, `expires_at`, `scope` and `raw` (the provider's profile JSON, untouched). Key your users by `(provider, uid)`: `email` and `login` can change. Link a sign-in to an existing account by email only when `email_verified == Some(true)`. For GitHub that means the primary, verified address from `/user/emails`, which needs the `user:email` scope.

`Identity`'s `Debug` output redacts the tokens, so logging it is safe.

### The routes

| Route | Does |
|---|---|
| `GET` or `POST /auth/{provider}` | Sets a ten-minute encrypted state cookie, redirects to the provider. `?return_to=/some/path` is where the user lands afterwards. |
| `GET /auth/{provider}/callback` | Checks the state, exchanges the code (with the PKCE verifier), fetches the profile, calls `on_sign_in`, starts a fresh session, redirects. |
| `GET` or `POST /auth/sign_out` | Clears the session, redirects to `after_sign_out_path`. |

After sign-in the user lands on the first of these that is set: the sign-in link's `return_to`, then `SignIn::redirect_to`, then `auth.oauth.after_sign_in_path`, then the page the sign-in started from (its `Referer`, see below), then `/`.

`return_to` accepts same-site paths only. `//evil.example`, `/\evil.example`, absolute URLs and anything with control characters are dropped, so a crafted sign-in link cannot bounce users off-site.

The parameter is named by `auth.oauth.origin_param` (OmniAuth's `origin_param`), which defaults to `auth.session.return_to_param`, which defaults to `return_to`. Rename one and the redirect `CurrentUser` makes and the parameter the sign-in route reads stay in step. Without the parameter, and when neither `SignIn::redirect_to` nor `after_sign_in_path` is set, the flow falls back to the `Referer`, like OmniAuth does, when it is a page on this site: same host as the request, or the origin of `auth.oauth.base_url`. A `Referer` from another site, or one under `/auth/` or at `sign_in_path` or `failure_path` (compared without query string or trailing slash), is ignored, so a sign-in link on the login page does not send users back to the login page. The `/auth/` prefix is fixed: it is where `oauth::routes` mounts. The parameter cannot be called `remember`, which is the remember-me flag; Loco refuses to boot with that name.

The state cookie (`loco_oauth_state_<provider>`, prefix set by `auth.oauth.state_cookie_name`) is encrypted, not just signed: it holds the PKCE verifier and the browser cannot read it.

### Adding your own routes next to them

`oauth::routes()` returns ordinary `Routes`, so add to them. A literal segment wins over `{provider}`, so a GitHub App install callback can live at `/auth/github/setup`:

```rust
pub fn routes() -> Routes {
    oauth::routes::<OAuth>().add("/github/setup", get(github_app_setup))
}

async fn github_app_setup(CurrentUser(user): CurrentUser<String>) -> Result<Response> {
    format::text(&format!("installed for {user}"))
}
```

### Failures

The user cancelling at the provider, an expired or forged state, a refused code and a failed profile call all end in `OAuthHandler::on_failure` with an `OAuthFailure { provider, kind, detail, retry_path }`. The state cookie is cleared every time, so a callback URL cannot be replayed.

By default this redirects to `auth.oauth.failure_path?error=<kind>&provider=<name>` when that is set. Otherwise it renders a minimal `401` page with the message and a "Try again" link. The `kind` values are `cancelled`, `provider_error`, `invalid_state`, `missing_code`, `token_exchange`, `profile`, `denied` and `configuration`, and each has a user-facing `kind.message()`. `detail` is meant for logs: it can contain text the provider or an attacker put in the URL. Override `on_failure` to render your own page. It gets the callback request's `Parts`, so it can read headers or the view engine, like Devise's failure app:

```rust
use axum::http::request::Parts;
use loco_rs::{auth::oauth::OAuthFailure, prelude::*};

pub struct OAuth;

#[async_trait]
impl OAuthHandler for OAuth {
    async fn on_sign_in(_ctx: &AppContext, identity: &Identity) -> Result<SignIn> {
        Ok(SignIn::new(identity.uid.clone()))
    }

    async fn on_failure(
        _ctx: &AppContext,
        parts: &Parts,
        failure: &OAuthFailure,
    ) -> Result<Response> {
        let Some(v) = parts.extensions.get::<ViewEngine<TeraView>>() else {
            return format::redirect(&format!("/login?error={}", failure.kind.as_str()));
        };
        format::render().view(
            &v.0,
            "auth/failure.html",
            data!({ "message": failure.kind.message(), "retry": failure.retry_path }),
        )
    }
}
```

## 4. Read the signed-in user

`CurrentUser<String>` gives you the id `on_sign_in` returned:

```rust
use loco_rs::prelude::*;

async fn dashboard(CurrentUser(user_id): CurrentUser<String>) -> Result<Response> {
    format::text(&format!("signed in as {user_id}"))
}

async fn home(MaybeCurrentUser(user_id): MaybeCurrentUser<String>) -> Result<Response> {
    format::text(user_id.as_deref().unwrap_or("anonymous"))
}
```

To get a model instead, implement `SessionUser` for it:

```rust
use loco_rs::{auth::session::SessionUser, prelude::*};

#[async_trait]
impl SessionUser for users::Model {
    async fn find_by_session_id(ctx: &AppContext, id: &str) -> Result<Option<Self>> {
        Ok(users::Entity::find()
            .filter(users::Column::Pid.eq(id))
            .one(&ctx.db)
            .await?)
    }
}

async fn profile(CurrentUser(user): CurrentUser<users::Model>) -> Result<Response> {
    format::json(user.name)
}
```

When nobody is signed in, `CurrentUser` answers `401`. A browser request is redirected to `auth.session.sign_in_path?return_to=<current path>` instead, when that path is set. Send that `return_to` on to `/auth/{provider}?return_to=...` and the user comes back where they started. A user that `find_by_session_id` no longer finds (deleted, say) counts as signed out. The lookup runs once per request, however many extractors ask for it.

What counts as a browser request is `auth.session.navigational_formats` (Devise's setting of the same name): the `Accept` types that get the redirect. The default is `[html]` (`text/html` or `application/xhtml+xml`). Add `turbo_stream` for Hotwire forms, or `"*/*"` to redirect requests that send no `Accept` at all. Other names are `json`, `xml` or any media type. The query parameter is `auth.session.return_to_param`.

For anything else, override `SessionUser::on_signed_out`, Devise's failure app. It gets the request and returns the response:

```rust
use axum::http::request::Parts;
use loco_rs::{auth::session::{self, SessionUser}, prelude::*};

#[async_trait]
impl SessionUser for users::Model {
    async fn find_by_session_id(ctx: &AppContext, id: &str) -> Result<Option<Self>> {
        Ok(users::Model::find_by_pid(&ctx.db, id).await.ok())
    }

    async fn on_signed_out(ctx: &AppContext, parts: &Parts) -> Response {
        if parts.uri.path().starts_with("/admin") {
            return format::redirect("/admin/login").into_response();
        }
        session::signed_out(parts, ctx)
    }
}
```

`session::signed_out` is the default behavior, so an override can fall back to it.

## The session and signed cookies on their own

`Session` works without OAuth, for example after a password sign-in:

```rust
use loco_rs::prelude::*;

async fn sign_in(session: Session) -> Result<impl IntoResponse> {
    // renew() drops anything planted before sign-in (session fixation).
    let session = session.renew().set_user_id("user-pid").insert("theme", &"dark")?;
    Ok((session, format::redirect("/")?))
}

async fn sign_out(session: Session) -> Result<impl IntoResponse> {
    Ok((session.clear(), format::redirect("/")?))
}
```

The session is a JSON map in one signed cookie. The browser can read it but not change it, so keep it to identifiers. It carries its own expiry, which the server enforces whatever the browser does with `Max-Age`. A session that would outgrow a cookie (about 3.8 KB) is an error, not a silent sign-out. Return the `Session` from the handler for changes to reach the browser. An unchanged session writes no cookie.

## Server-side sessions and remember me

A cookie session cannot be revoked: whoever holds the cookie is signed in until it expires. For apps that need "sign out everywhere", or a "remember me" box, keep the session on the server:

```yaml
auth:
  session:
    secret: <%= get_env(name="SESSION_SECRET") %>
    store: database        # or redis
    ttl: 86400             # optional, seconds an idle session lives (default 1 day)
    remember_for: 1209600  # optional, seconds (default 14 days); must exceed ttl
```

Nothing in your handlers changes. `Session`, `CurrentUser` and the OAuth routes work the same; the cookie now holds only a signed random id (256 bits), and the data, its expiry and the user id live in the store.

- The session cookie has no `Max-Age`: it ends with the browser. The server ends the session `ttl` after its last use (it slides, so an active user stays signed in). That is why `max_age` applies to the cookie store only, and why remember me, which outlives the browser, has to last longer than `ttl`.
- **`database`** creates `loco_sessions` and `loco_remember_tokens` the first time a session is used, if they are missing (Postgres and SQLite). The statements run in one transaction under an advisory lock on Postgres, so several processes starting together do not collide, and commands that never touch a session (migrations, tasks) never create them. Only the SHA-256 of the session id is stored, so a leaked table cannot be replayed. Expired rows read as absent and are deleted in batches as sessions are saved; call `SessionStore::purge_expired` from a task to sweep on a schedule.
- **`redis`** stores one key per session with a TTL, plus a sorted set per user (members scored by when they expire, dead ones swept on every write) so sign out everywhere finds every live session and series. Its scripts reach keys named by those sets, so it needs a single Redis node; Redis Cluster is not supported. It uses `auth.session.redis_uri`, or the `cache` Redis when that is unset.
- Signing in with `session.renew()` gives the user a new id, so an id planted before sign-in is dead. Setting a different user id with `set_user_id` rotates the id too, even if you forgot `renew()`. `session.clear()` deletes the record and the browser's remember me series, and stays cleared if you write to the session afterwards (a flash message, say).
- Writes to an existing session are updates, never inserts: a request that was in flight when the session was signed out or revoked cannot bring it back. The writes of one response (delete the old id, add the new one, rotate the remember series) are one transaction on the `database` store.
- Sign a user out of every device (sessions and remember-me tokens):

```rust
use loco_rs::{auth::session_store, prelude::*};

async fn sign_out_everywhere(
    State(ctx): State<AppContext>,
    CurrentUser(pid): CurrentUser<String>,
    session: Session,
) -> Result<impl IntoResponse> {
    session_store::sign_out_everywhere(&ctx, &pid).await?;
    Ok((session.clear(), format::redirect("/")?))
}
```

### Remember me

Call `.remember()?` on the session when the user ticks the box, or on the OAuth `SignIn`. The `/auth/{provider}` link also takes `?remember=1`:

```rust
let session = session.renew().set_user_id(pid).remember()?;
```

A second cookie (`loco_remember`, `HttpOnly`, same `Secure` and `SameSite` as the session) carries a series id and a token. The store keeps the series, the user and a generation counter; the token of each generation is an HMAC of the series under the cookie signing key, so a database dump alone cannot mint one. When a request has no live session but a valid remember cookie, the session is re-created and the token rotates (a compare-and-swap, so requests arriving together agree on one rotation).

A series has a fixed lifetime, `remember_for` from the sign-in that asked for it: using it never extends it. If a known series shows up with a token that is neither the current one nor the one just replaced, the cookie was copied, so everything that user has is revoked, every session and every series, because the sessions a thief already minted would otherwise live on. The replaced token stays good for ten seconds, and answering it hands back the current token rather than rotating again. Two tabs loading at once, or a rotation response that never reached the browser, therefore heal instead of looking like theft. The series ends with the sign-in it belongs to: signing out, or signing in as someone else, deletes it and the browser's remember cookie, and `remember()` is the only way to start a new one.

Remember me needs a server-side store. `.remember()` on the `cookie` store returns an error at the call. `?remember=1` on an OAuth link is ignored there, so the sign-in still succeeds. Setting `auth.session.remember_for` with `store: cookie`, or an explicit value that is not longer than `ttl`, or over 400 days (left unset it is 14 days, or `ttl` plus a day if that is longer), stops the app at boot with a message saying so. So do `ttl: 0` and a `remember_cookie_name` equal to `cookie_name`. A response that carries a rotated remember cookie keeps its `Set-Cookie` header, and is marked `Cache-Control: private`, even when the ETag middleware turns it into a `304`. A rotated token that never reached the browser because the request panicked or the write failed looks like a replayed one once the ten seconds pass, and signs that user out everywhere. That is by design.

Server-side stores rely on the `auth_session` middleware to write the session before the response goes out. It is in the default stack; an app that builds its own `Hooks::middlewares` must keep it. If the store write fails, the client gets a `500` and no session cookie.

For single cookies use `SignedCookies` (tamper-evident) or `EncryptedCookies` (also unreadable; feature `auth_session_encrypted`). Both are keyed off the same `auth.session.secret`:

```rust
use loco_rs::prelude::*;

async fn remember(cookies: SignedCookies) -> Result<impl IntoResponse> {
    let cookie = cookies.options().cookie("theme", "dark");
    Ok((cookies.add(cookie), "saved"))
}

async fn read(cookies: SignedCookies) -> Result<Response> {
    format::json(cookies.get("theme").map(|c| c.value().to_string()))
}
```

A tampered cookie, or one signed with a different secret, reads as absent. Every cookie Loco sets is `Path=/; HttpOnly; SameSite=Lax`, plus `Secure` when `server.host` is `https://` (override with `auth.session.secure`) and a `Max-Age`. The OAuth callback is a top-level navigation from the provider's site, and `SameSite=Strict` would drop the state cookie on it, which is why the setting is `Lax`.

## Production checklist

- **Set `auth.session.secret`** from the environment, with a long random value (`openssl rand -base64 64`). Without it, development and test sign cookies with a public default and log a warning. Production refuses (signed-cookie extractors answer `500`), and `cargo loco doctor --production` reports it, along with secrets shorter than 32 characters. Changing the secret signs everyone out.
- **Set `auth.oauth.base_url`** to your public origin, so callback URLs match what you registered with each provider.
- **Starting the flow with `POST`.** A `GET /auth/{provider}` lets any page sign a visitor in with whatever account their browser already has at the provider. For plain sign-in that does no harm. For *account linking*, where the identity is attached to the user who is already signed in, it does (OmniAuth's CVE-2015-9284). If you link accounts in `on_sign_in`, start the flow from a CSRF-protected `POST` form.
- `/auth/sign_out` answers `GET` for simple links. A third-party page can sign users out with it, which is a nuisance, not a breach. Serve sign-out from a `POST` form if that matters to you.

## Testing

The provider is just HTTP, so point its URLs at a fake server in `config/test.yaml`, or at one built in the test:

```yaml
auth:
  oauth:
    providers:
      github:
        client_id: test-client-id
        client_secret: test-client-secret
        base_url: http://127.0.0.1:4010   # your fake; api_url follows as /api/v3
```

Loco's own suite does exactly this (`tests/controller/oauth.rs`). An in-process axum fake on a random port issues codes bound to the PKCE challenge, and the test plays the browser between it and the app.

Most tests only need someone signed in, not the sign-in itself. `sign_in` from `loco_rs::testing::prelude` is Devise's `sign_in user`: it gives the test server a session for that id, on whichever session store the app uses, and later requests are signed in:

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
async fn shows_the_dashboard() {
    request::<App, _, _>(|mut request, ctx| async move {
        sign_in(&mut request, &ctx, "user-pid").await.unwrap();
        request.get("/dashboard").await.assert_status_ok();

        request.clear_cookies(); // signed out again
        request.get("/dashboard").await.assert_status_unauthorized();
    })
    .await;
}
```

The id is the one `on_sign_in` returned and `SessionUser::find_by_session_id` receives.
