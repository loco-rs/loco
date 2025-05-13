+++
title = "Views"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

In `Loco`, the processing of web requests is divided between a controller, model and view.

- **The controller** is handling requests parsing payload, and then control flows to models
- **The model** primarily deals with communicating with the database and executing CRUD operations when required. As well as modeling all business and domain logic and operations.
- **The view** takes on the responsibility of assembling and rendering the final response to be sent back to the client.

You can choose to have _JSON views_, which are JSON responses, or _Template views_ which are powered by a template view engine and eventually are HTML responses. You can also combine both.

<div class="infobox">
This is similar in spirit to Rails' `jbuilder` views which are JSON, and regular views, which are HTML, only that in LOCO we focus on being JSON-first.
</div>

## JSON views

As an example we have an endpoint that handles user login. When the user is valid we can pass the `user` model into the `LoginResponse` view (which is a JSON view) to return the response.

There are 3 steps:

1. Parse, accept the request
2. Create domain objects: models
3. Hand off the domain model to a view object which **shapes** the final response

The following Rust code represents a controller responsible for handling user login requests, which handes off _shaping_ of the response to `LoginResponse`.

```rust
use crate::{views::auth::LoginResponse};
async fn login(
    State(ctx): State<AppContext>,
    Json(params): Json<LoginParams>,
) -> Result<Response> {
    // Fetching the user model with the requested parameters
    // let user = users::Model::find_by_email(&ctx.db, &params.email).await?;

    // Formatting the JSON response using LoginResponse view
    format::json(LoginResponse::new(&user, &token))
}
```

On the other hand, `LoginResponse` is a response shaping view, which is powered by `serde`:

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

When you want to return HTML to the user, you use server-side templates. This is similar to how Ruby's `erb` works, or Node's `ejs`, or PHP for that matter.

For server-side templates rendering we provide the built in `TeraView` engine which is based on the popular [Tera](http://keats.github.io/tera/) template engine.

<div class="infobox">
To use this engine you need to verify that you have a <code>ViewEngineInitializer</code> in <code>initializers/view_engine.rs</code> which is also specified in your <code>app.rs</code>. If you used the SaaS Starter, this should already be configured for you.
</div>

The Tera view engine takes resources from the new `assets/` folder. Here is an example structure:

```
assets/
├── i18n
│   ├── de-DE
│   │   └── main.ftl
│   ├── en-US
│   │   └── main.ftl
│   └── shared.ftl
├── static
│   ├── 404.html
│   └── image.png
└── views
    └── home
        └── hello.html
config/
:
src/
├── controllers/
├── models/
:
└── views/
```

### Creating a new view

First, create a template. In this case we add a Tera template, in `assets/views/home/hello.html`. Note that **assets/** sits in the root of your project (next to `src/` and `config/`).

```html
<html>
  <body>
    find this tera template at <code>assets/views/home/hello.html</code>:
    <br />
    <br />
    {{ /* t(key="hello-world", lang="en-US") */ }},
    <br />
    {{ /* t(key="hello-world", lang="de-DE") */ }}
  </body>
</html>
```

Now create a strongly typed `view` to encapsulate this template in `src/views/dashboard.rs`:

```rust
// src/views/dashboard.rs
use loco_rs::prelude::*;

pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::render().view(&v, "home/hello.html", data!({}))
}

```

And add it to `src/views/mod.rs`:

```rust
pub mod dashboard;
```

Next, go to your controller and use the view:

```rust
// src/controllers/dashboard.rs
use loco_rs::prelude::*;

use crate::views;

pub async fn render_home(ViewEngine(v): ViewEngine<TeraView>) -> Result<impl IntoResponse> {
    views::dashboard::home(v)
}

pub fn routes() -> Routes {
    Routes::new().prefix("home").add("/", get(render_home))
}

```

Finally, register your new controller's routes in `src/app.rs`

```rust
pub struct App;
#[async_trait]
impl Hooks for App {
    // omitted for brevity

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::auth::routes())
            // include your controller's routes here
            .add_route(controllers::dashboard::routes())
    }
```

Once you've done all the above, you should be able to see your new routes when running `cargo loco routes`

```
$ cargo loco routes
[GET] /_health
[GET] /_ping
[POST] /api/auth/forgot
[POST] /api/auth/login
[POST] /api/auth/register
[POST] /api/auth/reset
[POST] /api/auth/verify
[GET] /api/auth/current
[GET] /home              <-- the corresponding URL for our new view
```

### How does it work?

- `ViewEngine` is an extractor that's available to you via `loco_rs::prelude::*`
- `TeraView` is the Tera view engine that we supply with Loco also available via `loco_rs::prelude::*`
- Controllers need to deal with getting a request, calling some model logic, and then supplying a view with **models and other data**, not caring about how the view does its thing
- `views::dashboard::home` is an opaque call, it hides the details of how a view works, or how the bytes find their way into a browser, which is a _Good Thing_
- Should you ever want to swap a view engine, the encapsulation here works like magic. You can change the extractor type: `ViewEngine<Foobar>` and everything works, because `v` is eventually just a `ViewRenderer` trait

### Static assets

If you want to serve static assets and reference those in your view templates, you can use the _Static Middleware_, configure it this way:

```yaml
static:
  enable: true
  must_exist: true
  precompressed: false
  folder:
    uri: "/static"
    path: "assets/static"
  fallback: "assets/static/404.html"
```

In your templates you can refer to static resources in this way:

```html
<img src="/static/image.png" />
```

However, for the static middleware to work, ensure that the default fallback is disabled:

```yaml
fallback:
  enable: false
```

### Customizing the Tera view engine

The Tera view engine comes with the following configuration:

- Template loading and location: `assets/**/*.html`
- Internationalization (i18n) configured into the Tera view engine, you get the translation function: `t(..)` to use in your templates

If you want to change any configuration detail for the `i18n` library, you can go and edit `src/initializers/view_engine.rs`.

By editing the initializer you can:

- Add custom Tera functions
- Remove the `i18n` library
- Change configuration for Tera or the `i18n` library
- Provide a new or custom, Tera (maybe a different version) instance

### Using your own view engine

If you do not like Tera as a view engine, or want to use Handlebars, or others you can create your own custom view engine very easily.

Here's an example for a dummy "Hello" view engine. It's a view engine that always returns the word _hello_.

```rust
// src/initializers/hello_view_engine.rs
use axum::{Extension, Router as AxumRouter};
use async_trait::async_trait;
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

### Tera Built-ins

Loco includes Tera with its [built-ins](https://keats.github.io/tera/docs/#built-ins) functions. In addition, Loco introduces the following custom built-in functions:

To see Loco built-in function:

- [numbers](https://docs.rs/loco-rs/latest/loco_rs/controller/views/tera_builtins/filters/number/index.html)

## Embedded Assets Feature

The Embedded Assets feature in Loco allows you to bundle all your static assets directly into your application binary. This means that everything under the `assets` folder, including CSS, images, PDFs, and more, becomes part of a single executable file.

To use this feature, you need to enable the `embedded_assets` feature when importing `loco-rs` in your `Cargo.toml`:

```toml
[dependencies]
loco-rs = { version = "...", features = ["embedded_assets"] }
```

### Benefits

- **Single Binary Deployment:** Simplifies deployment as you only need to distribute a single file. No need to worry about separate asset directories or CDN configurations for simpler deployments.
- **Atomic Updates:** When you update your application, the assets are updated atomically with the code, reducing the chances of mismatches between code and assets.
- **Potentially Faster Load Times:** Assets are loaded directly from memory, which can be faster than reading from the filesystem, especially in environments with slow disk I/O.

### Considerations

- **Increased Binary Size:** Embedding assets will naturally increase the size of your application binary.
- **Recompilation for Asset Changes:** Any change to an asset requires recompiling the application. This might slow down development workflows if assets are changed frequently.

### Seamlessly Switching Modes

You can easily switch between using embedded assets and serving assets from the filesystem without any code changes in your controllers or views. The switch is handled by the presence or absence of the `embedded_assets` feature flag.

However, to ensure Tera functions correctly when _not_ using embedded assets (i.e., serving from the filesystem), you need to ensure that your `src/initializers/view_engine.rs` file only contains the necessary Tera function registration if you had customized it previously. Specifically, for the translation function `t`, ensure your initializer looks like this if you are not using `loco_rs::tera_helpers::FluentLoader`:

```rust
tera_engine
    .tera
    .register_function("t", FluentLoader::new(arc));
```

Alternatively, you can introduce an internal feature flag within your application to toggle how assets are loaded or how Tera is configured, providing more granular control.

### Build Time Logs

When you build your application with the `embedded_assets` feature enabled, Loco will scan your `assets` directory and embed the discovered files. You will see logs similar to the following during the build process, indicating which assets are being included:

```
warning: loco-rs@0.15.0: Assets will only be loaded from the application directory
warning: loco-rs@0.15.0: Discovered directories for assets:
warning: loco-rs@0.15.0:   - /path/to/your/myapp/assets
warning: loco-rs@0.15.0:   - /path/to/your/myapp/assets/static
warning: loco-rs@0.15.0:   - /path/to/your/myapp/assets/i18n
warning: loco-rs@0.15.0:   - /path/to/your/myapp/assets/i18n/de-DE
warning: loco-rs@0.15.0:   - /path/to/your/myapp/assets/i18n/en-US
warning: loco-rs@0.15.0:   - /path/to/your/myapp/assets/views
warning: loco-rs@0.15.0:   - /path/to/your/myapp/assets/views/home
warning: loco-rs@0.15.0: Found asset: /path/to/your/myapp/assets/static/styles.css -> /static/styles.css
warning: loco-rs@0.15.0: Found asset: /path/to/your/myapp/assets/static/dummy.pdf -> /static/dummy.pdf
warning: loco-rs@0.15.0: Found asset: /path/to/your/myapp/assets/static/404.html -> /static/404.html
warning: loco-rs@0.15.0: Found asset: /path/to/your/myapp/assets/views/base.html -> base.html
warning: loco-rs@0.15.0: Found 13 asset files
warning: loco-rs@0.15.0: Generated code for 6 static assets and 7 templates
```

This output confirms that Loco has found your asset files (like CSS, PDFs, HTML templates) and has generated the necessary code to embed them into the binary. The paths will reflect your project's structure.
