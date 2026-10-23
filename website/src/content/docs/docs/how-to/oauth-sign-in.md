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
  oauth:
    # base_url: https://app.example.com   # public origin for callback URLs
    # after_sign_in_path: /
    # after_sign_out_path: /
    # failure_path: /login                # else a minimal error page
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
| `GET` or `POST /auth/{provider}` | Sets a ten-minute state cookie, redirects to the provider. `?return_to=/some/path` is where the user lands afterwards. |
| `GET /auth/{provider}/callback` | Checks the state, exchanges the code (with the PKCE verifier), fetches the profile, calls `on_sign_in`, starts a fresh session, redirects. |
| `GET` or `POST /auth/sign_out` | Clears the session, redirects to `after_sign_out_path`. |

After sign-in the user lands on the first of these that is set: the flow's `return_to`, then `SignIn::redirect_to`, then `auth.oauth.after_sign_in_path`, then `/`.

`return_to` accepts same-site paths only. `//evil.example`, `/\evil.example`, absolute URLs and anything with control characters are dropped, so a crafted sign-in link cannot bounce users off-site.

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

By default this redirects to `auth.oauth.failure_path?error=<kind>&provider=<name>` when that is set. Otherwise it renders a minimal `401` page with the message and a "Try again" link. The `kind` values are `cancelled`, `provider_error`, `invalid_state`, `missing_code`, `token_exchange`, `profile`, `denied` and `configuration`, and each has a user-facing `kind.message()`. `detail` is meant for logs: it can contain text the provider or an attacker put in the URL. Override `on_failure` to render your own page:

```rust
use loco_rs::{auth::oauth::OAuthFailure, prelude::*};

pub struct OAuth;

#[async_trait]
impl OAuthHandler for OAuth {
    async fn on_sign_in(_ctx: &AppContext, identity: &Identity) -> Result<SignIn> {
        Ok(SignIn::new(identity.uid.clone()))
    }

    async fn on_failure(_ctx: &AppContext, failure: &OAuthFailure) -> Result<Response> {
        format::redirect(&format!("/login?error={}", failure.kind.as_str()))
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

When nobody is signed in, `CurrentUser` answers `401`. A browser request (one that accepts `text/html`) is redirected to `auth.session.sign_in_path?return_to=<current path>` instead, when that path is set. Send that `return_to` on to `/auth/{provider}?return_to=...` and the user comes back where they started. A user that `find_by_session_id` no longer finds (deleted, say) counts as signed out. The lookup runs once per request, however many extractors ask for it.

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
