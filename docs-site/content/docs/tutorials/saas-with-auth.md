+++
title = "Build a Small Authenticated App"
description = "Generate an app from the SaaS starter path, register and log in a user, call a JWT-protected endpoint, then protect one of your own."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

Every Loco app generated with a database ships with a complete authentication suite: registration, login, email verification, password reset, magic links, and a JWT-protected "current user" endpoint — no extra generator, no starter template to hunt for. This lesson generates one, exercises the built-in auth flow end to end, then protects a resource of your own with the same JWT extractor the built-in endpoints use.

You should already be comfortable with the basics from [Your First App](@/docs/tutorials/your-first-app.md).

## 1. Generate the app

```sh
loco new --name saas_app --db sqlite --bg async --assets serverside
cd saas_app
```

This is the same combination of flags Loco's own `examples/demo` app in the loco-rs repository is generated with — server-rendered assets, SQLite, and in-process async workers. Choosing a database is what turns on authentication: `auth` and `mailer` scaffolding are both included automatically whenever `--db` is `sqlite` or `postgres`, regardless of which starter you picked interactively — there's no separate "SaaS" flag to remember.

Confirm the auth routes are already there, with nothing else generated yet:

```sh
$ cargo loco routes
...
[POST] /api/auth/register
[GET] /api/auth/verify/{token}
[POST] /api/auth/login
[POST] /api/auth/forgot
[POST] /api/auth/reset
[GET] /api/auth/current
[POST] /api/auth/magic-link
[GET] /api/auth/magic-link/{token}
[POST] /api/auth/resend-verification-mail
...
```

## 2. Skip the SMTP dependency

Registering a user sends a welcome email through the configured mailer. `config/development.yaml` enables SMTP against `localhost:1025` by default, which means registration will fail with a 500 unless you either run a local SMTP catcher there, or tell the mailer to stub outgoing mail instead of sending it. For this lesson, stub it — open `config/development.yaml` and add `stub: true` under `mailer`:

```yaml
mailer:
  stub: true
  smtp:
    enable: true
    host: localhost
    # ...
```

<div class="infobox">
The generated <code>config/test.yaml</code> already sets <code>mailer.stub: true</code> — that's why your app's tests never need a live mailbox. You're applying the same setting to <code>development.yaml</code> so <code>cargo loco start</code> behaves the same way.
</div>

## 3. Start the app

```sh
cargo loco start
```

## 4. Register a user

```sh
$ curl --location 'localhost:5150/api/auth/register' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "name": "Loco user",
         "email": "user@loco.rs",
         "password": "12341234"
     }'
{}
```

An empty `{}` on success is intentional: the endpoint always answers the same way whether or not the email was already registered, so no request can be used to probe your user list.

## 5. Log in

```sh
$ curl --location 'localhost:5150/api/auth/login' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

```json
{
  "token": "eyJhbGciOiJIUzUxMiJ9...",
  "pid": "2b20f998-b11e-4aeb-96d7-beca7671abda",
  "name": "Loco user",
  "is_verified": false
}
```

`is_verified` is `false` because you haven't clicked the (stubbed, unsent) verification email — that's fine, **login doesn't require a verified email**, only a matching password. Save the `token`; every authenticated request below uses it as a bearer token.

## 6. Call the built-in protected endpoint

```sh
$ curl --location 'localhost:5150/api/auth/current' \
     --header 'Authorization: Bearer TOKEN'
```

```json
{
  "pid": "2b20f998-b11e-4aeb-96d7-beca7671abda",
  "name": "Loco user",
  "email": "user@loco.rs"
}
```

Under the hood, `current` is nothing special — it's a normal handler that takes `auth::JWT` as its first argument:

```rust
async fn current(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(CurrentResponse::new(&user))
}
```

If the `Authorization` header is missing, malformed, or carries an expired/invalid token, axum never reaches your handler body — the `auth::JWT` extractor itself rejects the request with `401 Unauthorized`. Try it without the header to see that happen.

## 7. Protect a resource of your own

The pattern above works for any handler, not just the built-in ones. Generate a `notes` scaffold:

```sh
$ cargo loco generate scaffold notes title:string content:text
```

Open `src/controllers/notes.rs` and change the `add` handler's signature to also require `auth::JWT`:

```rust
pub async fn add(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<Params>,
) -> Result<Response> {
    // we only need to know the request carries a valid, known user
    let _current_user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    let mut item = ActiveModel { ..Default::default() };
    params.update(&mut item);
    let item = item.insert(&ctx.db).await?;
    format::json(item)
}
```

`auth::JWT` is already in scope through `loco_rs::prelude::*`, which every generated controller imports. Restart the app and confirm the two behaviors:

```sh
# no token: rejected before your handler even runs
$ curl -X POST -H "Content-Type: application/json" \
    -d '{"title":"secret","content":"shh"}' localhost:5150/api/notes
# 401 Unauthorized

# with token: goes through
$ curl -X POST -H "Content-Type: application/json" \
    -H "Authorization: Bearer TOKEN" \
    -d '{"title":"secret","content":"shh"}' localhost:5150/api/notes
{"id":1,"created_at":"...","updated_at":"...","title":"secret","content":"shh"}
```

`list`, `get_one`, `update`, and `remove` on `notes` are still open to anyone — add `auth: auth::JWT` to their signatures the same way if you want the whole resource locked down.

## What's actually enforced, and what isn't

- The JWT secret and expiration live in `config/development.yaml` under `auth.jwt`. Every environment (`development`, `test`, `production`) gets its own generated secret — never share one across environments. See the [Configuration reference](@/docs/reference/configuration.md) for every key under `auth:`.
- Tokens are signed HS512 by default, and the configured secret must be valid base64 — this is handled for you in the generated config, but matters if you ever hand-roll one.
- `auth::JWT` only checks that the token is valid and unexpired; it does not check `is_verified`. If your app needs "must have verified their email" as a business rule, check `user.email_verified_at.is_some()` yourself inside the handler, the same way you looked up the user by `pid` above.

## Next

- [Protect a Route with JWT](@/docs/how-to/jwt-auth.md) — the full endpoint-by-endpoint reference: forgot/reset password, email verification, magic links, and API-key auth as an alternative to JWTs.
- [Configuration reference](@/docs/reference/configuration.md) — every `auth:` and `mailer:` YAML key.
- [The Tour](@/docs/tutorials/the-tour.md) — if you haven't yet, see models, workers, and tasks covered end to end.
- [Add a model](@/docs/how-to/add-model.md) — keep building out `notes` (or your own resource) with relations and validation.
