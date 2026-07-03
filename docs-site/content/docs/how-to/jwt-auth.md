+++
title = "Protect a Route with JWT"
description = "Add JWT authentication to a route with the JWT and JWTWithUser<T> extractors, implement the Authenticable contract, and generate tokens."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 40
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/extras/authentication/"]

[extra]
lead = ""
toc = true
top = false
+++

Goal: require a valid JWT on a handler, and issue tokens your clients can send back.

Loco ships two axum extractors for JWT-protected routes:

- `auth::JWT` — validates the token and gives you the claims. Works without a database.
- `auth::JWTWithUser<T>` — validates the token **and** loads the user record from the database via your model's [`Authenticable`](#the-authenticable-contract) implementation. Requires the `with-db` feature.

Both live in `loco_rs::controller::extractor::auth` and are re-exported through `loco_rs::prelude::*` whenever the `auth_jwt` feature is on.

If you generated your app with `loco new` and picked a database-backed starter, `with-db` and `auth_jwt` are already on by default — see the [feature flags reference](@/docs/reference/feature-flags.md) if you need to check or change that.

## 1. Enable the `auth_jwt` feature

`auth_jwt` is a **default** feature (`Cargo.toml`): it pulls in `jsonwebtoken` with its pure-Rust `rust_crypto` backend, so no C toolchain is required even in a minimal build. If you depend on `loco-rs` with `default-features = false`, add it back explicitly:

```toml
loco-rs = { version = "...", default-features = false, features = ["auth_jwt", "with-db"] }
```

## 2. Configure the secret and expiration

Add an `auth.jwt` block to `config/development.yaml` (and every other environment file):

```yaml
auth:
  jwt:
    secret: "{{ get_env(name='JWT_SECRET') }}" # required, must be valid base64
    expiration: 604800 # required, seconds (7 days)
```

Two facts that will save you a confusing error message:

- **The secret must be valid base64.** Loco encodes/decodes tokens with `EncodingKey::from_base64_secret` / `DecodingKey::from_base64_secret`. A plain, non-base64 string doesn't fail at config-load time — it fails later, when a token is generated or validated, with an error that doesn't obviously point at the config. Generate a base64 secret, e.g. `openssl rand -base64 64`, and inject it via `get_env` as above rather than hardcoding it.
- **The default signing algorithm is HS512**, not HS256. It's set in code (`Algorithm::HS512`) and isn't a YAML key — override it only from Rust, via `JWT::algorithm(..)` when constructing the signer.

For the full `auth.jwt` key reference, including `location`, see the [configuration reference](@/docs/reference/configuration.md#auth). Token location (`Bearer` / `Query` / `Cookie`) is covered in its own guide: [Configure where Loco looks for the JWT](@/docs/how-to/jwt-locations.md).

## 3. Generate a token

Use `loco_rs::auth::jwt::JWT` (the signer/validator — not to be confused with the extractor of the same name below) to mint a token, typically from a login handler, using the secret and expiration already loaded on `AppContext`:

```rust
use loco_rs::prelude::*;

async fn login(State(ctx): State<AppContext>, /* ... */) -> Result<Response> {
    // `user` is whatever you loaded/verified during login (see hash-passwords.md).
    let jwt_config = ctx.config.get_jwt_config()?;

    let token = loco_rs::auth::jwt::JWT::new(&jwt_config.secret)
        .generate_token(jwt_config.expiration, user.pid.to_string(), serde_json::Map::new())
        .map_err(|e| Error::string(&e.to_string()))?;

    format::json(serde_json::json!({ "token": token }))
}
```

`generate_token` takes the expiration (seconds), the `pid` that becomes `claims.pid`, and an optional `serde_json::Map` of custom claims that get flattened alongside `pid` in the token. It returns a `jsonwebtoken` result, not Loco's `Result`, so map the error explicitly as shown.

## 4. Protect a route: claims only

Add `auth::JWT` as a handler parameter. Axum runs it as an extractor before your handler body executes; if the token is missing, unparsable, in the wrong location, or expired, the request never reaches your code — Loco returns `401 Unauthorized` for you.

```rust
use loco_rs::prelude::*;
use loco_rs::controller::extractor::auth;

async fn current(
    auth: auth::JWT,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(serde_json::json!({ "pid": auth.claims.pid }))
}
```

`auth.claims` is `UserClaims { pid, claims, .. }` — use `auth.claims.pid` for the subject, and `auth.claims.claims` for any custom claims you flattened in at generation time. This extractor needs no database, so it works even in DB-less apps.

## 5. Protect a route: claims + loaded user

When the handler needs the actual user row (not just the `pid`), use `auth::JWTWithUser<T>` where `T` is your user model. This requires the `with-db` feature and requires `T` to implement `Authenticable`.

```rust
use loco_rs::prelude::*;
use loco_rs::controller::extractor::auth;

async fn current(
    auth: auth::JWTWithUser<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(&auth.user)
}
```

Internally, `JWTWithUser` validates the token exactly like `auth::JWT`, then calls `T::find_by_claims_key(&ctx.db, &claims.pid)` to load the user. A database miss becomes `401 Unauthorized`; a database error becomes `500 Internal Server Error`.

## The `Authenticable` contract

Both `JWTWithUser<T>` and `ApiToken<T>` (see the [API-key guide](@/docs/how-to/api-key-auth.md)) require your user model to implement `loco_rs::model::Authenticable`:

```rust
pub trait Authenticable: Clone {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self>;
    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self>;
}
```

A typical Sea-ORM implementation looks up the row by the relevant column and maps a miss to `ModelError::EntityNotFound`:

```rust
impl Authenticable for super::_entities::users::Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = super::_entities::users::Entity::find()
            .filter(super::_entities::users::Column::ApiKey.eq(api_key))
            .one(db)
            .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        let user = super::_entities::users::Entity::find()
            .filter(super::_entities::users::Column::Pid.eq(claims_key))
            .one(db)
            .await?;
        user.ok_or(ModelError::EntityNotFound)
    }
}
```

**Note on scope:** the framework defines the `Authenticable` trait and the extractors above, but it does not generate a users model or `/api/auth/*` controllers for you — `loco-gen` has no auth/user template. That register/login/verify/reset-password flow, including the `Authenticable` implementation shown above, ships in the **SaaS starter** template (`loco new` → "SaaS app (with DB and user auth)"), which lives outside this repository. Use the pattern above as a starting point if you're wiring auth into a custom model.

## Verify it works

```sh
curl --location '127.0.0.1:5150/api/some/protected/route' \
     --header 'Authorization: Bearer <TOKEN>'
```

A missing, malformed, or expired token returns `401 Unauthorized`. A valid token reaches your handler with `auth.claims` (and, for `JWTWithUser`, `auth.user`) populated.

## Related

- [Configure where Loco looks for the JWT](@/docs/how-to/jwt-locations.md) — Bearer / Query / Cookie, single or multiple.
- [Protect a route with an API key](@/docs/how-to/api-key-auth.md) — `ApiToken<T>`, a separate always-Bearer-header mechanism.
- [Hash and verify passwords](@/docs/how-to/hash-passwords.md) — for the login handler that issues the token above.
- [Configuration reference](@/docs/reference/configuration.md#auth) — every `auth.jwt` key.
- [Feature flags reference](@/docs/reference/feature-flags.md) — `auth_jwt` / `with-db` defaults and interactions.
- [AppContext & prelude reference](@/docs/reference/app-context.md) — what `loco_rs::prelude::*` brings in under `auth_jwt`.
