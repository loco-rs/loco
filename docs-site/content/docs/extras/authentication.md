+++
title = "Authentication"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

## User Password Authentication

`Loco` simplifies the user authentication process, allowing you to set up a new website quickly. This feature not only saves time but also provides the flexibility to focus on crafting the core logic of your application.

### Authentication Configuration

The `auth` feature comes as a default with the library. If desired, you can turn it off and handle authentication manually.

### Getting Started with a SaaS App

Create your app using the [loco cli](/docs/getting-started/tour) and select the `SaaS app (with DB and user auth)` option.

To explore the out-of-the-box auth controllers, run the following command:

```sh
$ cargo loco routes
 .
 .
 .
[POST] /api/auth/forgot
[POST] /api/auth/login
[POST] /api/auth/register
[POST] /api/auth/reset
[GET] /api/auth/verify
[GET] /api/auth/current
 .
 .
 .
```

### Registering a New User

The `/api/auth/register` endpoint creates a new user in the database with an `email_verification_token` for account verification. A welcome email is sent to the user with a verification link.

##### Example Curl Request:

```sh
curl --location '127.0.0.1:5150/api/auth/register' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "name": "Loco user",
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

For security reasons, if the user is already registered, no new user is created, and a 200 status is returned without exposing user email details.

### Login

After registering a new user, use the following request to log in:

##### Example Curl Request:

```sh
curl --location '127.0.0.1:5150/api/auth/login' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

The response includes a JWT token for authentication, user ID, name, and verification status.

```sh
{
    "token": "...",
    "pid": "2b20f998-b11e-4aeb-96d7-beca7671abda",
    "name": "Loco user",
    "is_verified": false
}
```

- **Token**: A JWT token enabling requests to authentication endpoints. Refer to the [configuration documentation](@/docs/the-app/your-project.md#your-app-configuration) to customize the default token expiration and ensure that the secret differs between environments.
- **pid** - A unique identifier generated when creating a new user.
- **Name** - The user's name associated with the account.
- **Is Verified** - A flag indicating whether the user has verified their account.

### Account Verification

Upon user registration, an email with a verification link is sent. Visiting this link updates the `email_verified_at` field in the database, changing the `is_verified` flag in the login response to true.

#### Example Curl request:

```sh
curl --location --request GET '127.0.0.1:5150/api/auth/verify/TOKEN' \
     --header 'Content-Type: application/json'
```

### Reset Password Flow

#### Forgot Password

The `forgot` endpoint requires only the user's email in the payload. An email is sent with a reset password link, and a `reset_token` is set in the database.

##### Example Curl request:

```sh
curl --location '127.0.0.1:5150/api/auth/forgot' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "email": "user@loco.rs"
     }'
```

#### Reset Password

To reset the password, send the token generated in the `forgot` endpoint along with the new password.

##### Example Curl request:

```sh
curl --location '127.0.0.1:5150/api/auth/reset' \
     --header 'Content-Type: application/json' \
     --data '{
         "token": "TOKEN",
         "password": "new-password"
     }'
```

### Get current user

This endpoint is protected by auth middleware.

```sh
curl --location --request GET '127.0.0.1:5150/api/auth/current' \
     --header 'Content-Type: application/json' \
     --header 'Authorization: Bearer TOKEN'
```

### Creating an Authenticated Endpoint

To establish an authenticated endpoint, import `controller::middleware` from the `loco_rs` library and incorporate the auth middleware into the function endpoint parameters.

Consider the following example in Rust:

```rust
use axum::{extract::State, Json};
use loco_rs::{
    app::AppContext,
    controller::middleware,
    Result,
};

async fn current(
    auth: middleware::auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    /// Some response
}

```

## API Authentication

### Creating new app

For this time, let create your rest app using the [loco cli](/docs/getting-started/tour) and select the `Rest app` option.
To create new app, run the following command and follow the instructions:

```sh
$ loco new
```

To explore the out-of-the-box auth controllers, run the following command:

```sh
$ cargo loco routes
 .
 .
 .
[POST] /api/auth/forgot
[POST] /api/auth/login
[POST] /api/auth/register
[POST] /api/auth/reset
[GET] /api/auth/verify
[GET] /api/auth/current
 .
 .
 .
```

### Registering new user

The `/api/auth/register` endpoint creates a new user in the database with an `api_key` for request authentication. `api_key` will be used for authentication in the future requests.

#### Example Curl Request:

```sh
curl --location '127.0.0.1:5150/api/auth/register' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "name": "Loco user",
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

After registering a new user, make sure you see the `api_key` in the database for the new user.

### Creating an Authenticated Endpoint with API Authentication

To set up an API-authenticated endpoint, import `controller::middleware` from the loco_rs library and include the auth middleware in the function endpoint parameters using `middleware::auth::ApiToken`.

Consider the following example in Rust:

```rust
use loco_rs::prelude::*;
use loco_rs::controller::middleware;
use crate::{models::_entities::users, views::user::CurrentResponse};

async fn current_by_api_key(
    auth: middleware::auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(CurrentResponse::new(&auth.user))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("user")
        .add("/current-api", get(current_by_api_key))
}
```

### Requesting an API Authenticated Endpoint

To request an authenticated endpoint, you need to pass the `API_KEY` in the `Authorization` header.

#### Example Curl Request:

```sh
curl --location '127.0.0.1:5150/api/user/current-api' \
     --header 'Content-Type: application/json' \
     --header 'Authorization: Bearer API_KEY'
```

If the `API_KEY` is valid, you will get the response with the user details.
