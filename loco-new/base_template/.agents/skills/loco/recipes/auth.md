# Recipe: authentication

Rails uses `before_action :authenticate_user!`. Loco has no inheritance, so
authentication is an **extractor in the handler signature**. The type system
enforces it: a handler that takes `auth::JWT` cannot run unauthenticated.

Requires the `auth` feature (on by default). `auth` comes from
`loco_rs::prelude::*`.

## There are two types named `JWT` — do not mix them up

| Type | What it is |
|---|---|
| `loco_rs::controller::extractor::auth::JWT` | the **extractor** you put in a handler signature (`auth::JWT`) |
| `loco_rs::auth::jwt::JWT` | the **config/signing** type — `new(secret)`, `generate_token(...)`, `validate(...)` |

In app code you almost always want the first, reached as `auth::JWT`.

## Protect a handler

```rust
use loco_rs::prelude::*;

#[debug_handler]
async fn current(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(UserResponse::from(&user))
}
```

`auth.claims` is `UserClaims` — `pid` plus any custom claims you signed. The
extractor rejects the request with 401 before your body runs, so there is no
"if not logged in" branch to write.

Three extractors:

| Extractor | Use |
|---|---|
| `auth::JWT` | you only need the claims |
| `auth::JWTWithUser<T>` | you need the loaded user; `T: Authenticable` |
| `auth::ApiToken<T>` | API-key auth via the user's `api_key` |

**Every auth extractor requires `State<AppContext>` in the same signature**,
even when the handler never touches `ctx`. Without it Axum cannot infer the
router's state type and the build fails with a message that names neither auth
nor Loco:

```
the trait bound `AppContext: FromRef<()>` is not satisfied
```

So bind it and ignore it:

```rust
#[debug_handler]
async fn whoami(
    auth: auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(MachineIdentity::from(&auth.user))
}
```

`JWTWithUser` saves you the lookup:

```rust
async fn current(
    auth: auth::JWTWithUser<users::Model>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    format::json(UserResponse::from(&auth.user))
}
```

## Make your model authenticable

The generated auth scaffold does this for you. It is what lets the extractor
find a user:

```rust
#[async_trait]
impl Authenticable for Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        // ...
    }
    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        Self::find_by_pid(db, claims_key).await
    }
}
```

## Issue a token

```rust
let jwt_secret = ctx.config.get_jwt_config()?;
let token = user
    .generate_jwt(&jwt_secret.secret, jwt_secret.expiration)
    .or_else(|_| unauthorized("unauthorized!"))?;
```

## Passwords

Use `loco_rs::hash` — never add `argon2`, `bcrypt`, or `rand`:

```rust
let hashed = hash::hash_password(&params.password)?;
let valid  = hash::verify_password(&params.password, &user.password);
```

`hash::random_string(len)` covers token generation.

Hashing belongs on the model (`Model::create_with_password`), not in a handler.

## Configuration

```yaml
auth:
  jwt:
    secret: <%= get_env(name="JWT_SECRET", default="...") %>
    expiration: 604800    # seconds
```

Secrets arrive through config env-interpolation. Never `std::env::var` in
handler code.

Token location is configurable — bearer header (default), cookie, or query
param. See the docs under **How-to → JWT locations** rather than parsing headers
yourself.

## Scaffold the whole thing

A new app created with the SaaS/auth starter already ships registration, login,
verification, forgot/reset password, and magic link — with the model methods,
mailers, and request tests. Read `examples/demo/src/controllers/auth.rs` and
`examples/demo/src/models/users.rs` before writing auth by hand; they are the
reference implementation of every pattern in this file.
