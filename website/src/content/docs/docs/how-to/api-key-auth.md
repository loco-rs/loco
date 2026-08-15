---
title: Protect a Route with an API Key
description: "Authenticate requests with a per-user API key using the ApiToken<T> extractor and Authenticable::find_by_api_key."
sidebar:
  order: 41
---

Goal: authenticate a request with a long-lived, per-user API key instead of a JWT — useful for machine-to-machine or CLI clients that shouldn't have to re-authenticate for a fresh token.

Loco provides `auth::ApiToken<T>`, an axum extractor that reads a key from the `Authorization` header and loads the matching user via your model's `Authenticable::find_by_api_key`.

## Prerequisites

- The `with-db` feature (on by default) — `ApiToken<T>` is compiled only under `#[cfg(feature = "with-db")]`.
- Your user model implements `loco_rs::model::Authenticable`, in particular `find_by_api_key`. See [the `Authenticable` contract](/docs/how-to/jwt-auth#the-authenticable-contract) for the trait shape and an example implementation.
- A column on your user model to store the key (e.g. `api_key`), and a way to populate it — `loco_rs::hash::random_string` is a convenient generator; see [Hash and verify passwords](/docs/how-to/hash-passwords#generate-random-tokens).

Unlike JWT auth, `ApiToken<T>` needs **no `auth.jwt` configuration at all** — it doesn't call into `auth.jwt.secret`/`expiration`/`location`. It only needs a database and an `Authenticable` implementation.

## 1. Implement `find_by_api_key`

```rust
use async_trait::async_trait;
use loco_rs::model::{Authenticable, ModelError, ModelResult};
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter};

#[async_trait]
impl Authenticable for super::_entities::users::Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = super::_entities::users::Entity::find()
            .filter(super::_entities::users::Column::ApiKey.eq(api_key))
            .one(db)
            .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        // Required by the trait even if this app only uses ApiToken.
        // See jwt-auth.md if you also support JWTWithUser<T>.
        unimplemented!()
    }
}
```

## 2. Add `ApiToken<T>` to a handler

```rust
use loco_rs::prelude::*;
use loco_rs::controller::extractor::auth;

async fn current_by_api_key(
    auth: auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(&auth.user)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("user")
        .add("/current-api", get(current_by_api_key))
}
```

`auth.user` is the fully loaded `T` (your `Authenticable` model) — there's no separate claims struct to unwrap, unlike the JWT extractors.

## 3. Send the key as a Bearer token

**`ApiToken<T>` always reads the key from the `Authorization: Bearer <key>` header, and only from there.** This is a hard-coded read (`extract_token_from_header`), independent of any `auth.jwt.location` setting — the `location` config (`Bearer`/`Query`/`Cookie`, described in [Configure where Loco looks for the JWT](/docs/how-to/jwt-locations)) applies to the `JWT`/`JWTWithUser` extractors only, never to `ApiToken`.

```sh
curl --location '127.0.0.1:5150/api/user/current-api' \
     --header 'Authorization: Bearer <API_KEY>'
```

## Verify it works

- A valid, known key returns the user (200, with your handler's JSON body).
- An unknown key returns `401 Unauthorized` (the model lookup misses, mapped from `ModelError::EntityNotFound`).
- A database error while looking up the key returns `500 Internal Server Error`, and is logged via `tracing::error!`.

## Related

- [Protect a route with JWT](/docs/how-to/jwt-auth) — the `Authenticable` contract in full, plus the `JWT` / `JWTWithUser<T>` extractors.
- [Configure where Loco looks for the JWT](/docs/how-to/jwt-locations) — applies to JWT auth, not to `ApiToken`.
- [Hash and verify passwords](/docs/how-to/hash-passwords) — generate the random key value to store per user.
- [Feature flags reference](/docs/reference/feature-flags) — `with-db` default and what it gates.
