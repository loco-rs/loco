+++
title = "Views"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 14
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

In `Loco`, the processing of web requests is divided between a controller, model and view.

* **The controller** is handling requests parsing payload, and then control flows to models
* **The model** primarily deals with communicating with the database and executing CRUD operations when required. As well as modeling all business and domain logic and operations.
* **The view** takes on the responsibility of assembling and rendering the final response to be sent back to the client. 
 

You can choose to have _JSON views_, which are JSON responses, or _Template views_ which are powered by a template view engine and eventually are HTML responses. You can also combine both.

<div class="infobox">
This is similar in spirit to Rails' `jbuilder` views which are JSON, and regular views, which are HTML, only that in LOCO we focus on being JSON-first.
</div>

## JSON views

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

## Template views

Loco has a _view engine infrastructure_ built in. It means that you can take any kind of a template engine like Tera and Liquid, implement a `TemplateEngine` trait and use those in your controllers.

We provide a built in `TeraView` engine which requires no coding, it's ready use. To use this engine you need to verify that you have a `ViewEngineInitializer` in `initializers/view_engine.rs` which is also specified in your `app.rs`. If you used the SaaS Starter, this should already be configured for you.

<div class="infobox">
<b>NOTE: The SaaS starter includes a fully configured Tera view engine</b>, which includes an i18n library and asset loading, which is also wired into your app hooks
</div>

### Customizing the view engine

Out of the box, the Tera view engine comes with the following configured:

* Template loading and location: `assets/**/*.html`
* Internationalization (i18n) configured into the Tera view engine, you get the translation function: `t(..)` to use in your templates

If you want to change any configuration detail for the `i18n` library, you can go and edit `src/initializers/view_engine.rs`.

You can also add custom Tera functions in the same initializer.

### Creating a new view

First, create a template. In this case we add a Tera template, in `assets/views/home/hello.html`. Note that **assets/** sits in the root of your project (next to `src/` and `config/`).

```html
<html><body>
find this tera template at <code>assets/views/home/hello.html</code>: 
<br/>
<br/>
{{ t(key="hello-world", lang="en-US") }}, 
<br/>
{{ t(key="hello-world", lang="de-DE") }}

</body></html>
```

Now create a strongly typed `view` to encapsulate this template in `src/views/dashboard.rs`:

```rust
// src/views/dashboard.rs
pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::render().view(&v, "home/hello.html", json!({}))
}
```

And add it to `src/views/mod.rs`:

```rust
pub mod dashboard;
```

Finally, go to your controller and use the view:


```rust
// src/controllers/dashboard.rs
pub async fn render_home(ViewEngine(v): ViewEngine<TeraView>) -> Result<impl IntoResponse> {
    views::dashboard::home(v)
}
```

### How does it work?

* `ViewEngine` is an extractor that's available to you via `loco_rs::prelude::*`
* `TeraView` is the Tera view engine that we supply with Loco also available via `loco_rs::prelude::*`
* Controllers need to deal with getting a request, calling some model logic, and then supplying a view with **models and other data**, not caring about how the view does its thing
* `views::dashboard::home` is an opaque call, it hides the details of how a view works, or how the bytes find their way into a browser, which is a _Good Thing_
* Should you ever want to swap a view engine, the encapsulation here works like magic. You can change the extractor type: `ViewEngine<Foobar>` and everything works, because `v` is eventually just a `ViewRenderer` trait

## Using your own view engine

If you do not like Tera as a view engine, or want to use Handlebars, or others you can create your own custom view engine very easily.

Here's an example for a dummy "Hello" view engine. It's a view engine that always returns the word _hello_.

```rust
// src/initializers/hello_view_engine.rs
use axum::{async_trait, Extension, Router as AxumRouter};
use loco_rs::{
    app::{AppContext, Initializer},
    controller::views::{ViewEngine, ViewRenderer},
    Result,
};
use serde::Serialize;

#[derive(Clone)]
pub struct HelloView;
impl ViewRenderer for HelloView {
    fn render<S: Serialize>(&self, _key: &str, _data: S) -> Result<String> {
        Ok("hello".to_string())
    }
}

pub struct HelloViewEngineInitializer;
#[async_trait]
impl Initializer for HelloViewEngineInitializer {
    fn name(&self) -> String {
        "custom-view-engine".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router.layer(Extension(ViewEngine::from(HelloView))))
    }
}
```

To use it, you need to add it to your `src/app.rs` hooks:


```rust
// src/app.rs
// add your custom "hello" view engine in the `initializers(..)` hook
impl Hooks for App {
    // ...
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![
            // ,.----- add it here
            Box::new(initializers::hello_view_engine::HelloViewEngineInitializer),
        ])
    }
    // ...
```
