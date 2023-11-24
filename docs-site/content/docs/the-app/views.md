+++
title = "Views"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 5
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

In `Loco`, the processing of web requests is divided between action controller, action view and action model. action model primarily deals with communicating with the database and executing CRUD operations when required. Action controller is handling requests parsing payload, On the other hand Action View takes on the responsibility of assembling and rendering the final response to be sent back to the client. This separation of concerns allows for a clear and organized handling of the request-response lifecycle in a Rails application.

For an examples, we have an endpoint that handling user login request. in this case we creating an [controller](@/docs/the-app/controller.md) the defined the user payload and parsing in into the model for check if the user request is valid.
When the user is valid we can pass the `user` model into the `auth` view which take the user and parsing the relavant detatils that we want to return in the request.

Upon confirming the validity of the user, we pass the user model to the auth view. The auth view then takes the user and processes the relevant details that we intend to include in the response. This division of responsibilities allows for a clear and structured flow in handling user login requests within the application.

The following Rust code represents a controller responsible for handling user login requests

```rust
use crate::{views::auth::LoginResponse};
async fn login(
    State(ctx): State<AppContext>,
    Json(params): Json<LoginParams>,
) -> Result<Json<LoginResponse>> {

    // Fetching the user model with the requested parameters
    // let user = users::Model::find_by_email(&ctx.db, &params.email).await?;

    // Formatting the JSON response ussing LoginResponse view
    format::json(LoginResponse::new(&user, &token))
}
```

The Rust code below represents a view responsible for generating a structured response for user login. It uses the LoginResponse structure, and this is the response which returns to the user

```rust
use serde::{Deserialize, Serialize};

use crate::models::_entities::users;

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub pid: String,
    pub name: String,
}

impl LoginResponse {
    #[must_use]
    pub fn new(user: &users::Model, token: &String) -> Self {
        Self {
            token: token.to_string(),
            pid: user.pid.to_string(),
            name: user.name.clone(),
        }
    }
}

```
