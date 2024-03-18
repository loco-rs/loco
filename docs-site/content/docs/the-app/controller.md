+++
title = "Controller"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 13
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

`Loco` is a framework that wraps around [axum](https://crates.io/crates/axum), offering a straightforward approach to manage routes, middlewares, authentication, and more right out of the box. At any point, you can leverage the powerful axum Router and extend it with your custom middlewares and routes.

### Router Capabilities

`Loco` router provides several capabilities:

#### Defining App Routes in the App Hook

In the example below, multiple controllers are added to your app within the `AppRouter`. During initialization, you can choose between:

- **AppRoutes::with_default_routes():** Adds default loco endpoints like ping or heathy.
- **AppRoutes::empty():** Creates an empty router without default routes.

```rust
pub struct App;
#[async_trait]
impl Hooks for App {
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::foo::routes())
            .add_route(controllers::bar::routes())
    }
    ...
}
```

#### Adding a Prefix to All Routes

You can add a prefix URL to all your routes by providing the prefix to the AppRouter instance.

#### Adding extra state

Your app context and state is held in `AppContext` and is what Loco provides and sets up for you. There are cases where you'd want to load custom data,
logic, or entities when the app starts and be available to use in all controllers.

You could do that by using Axum's `Extension`. Here's an example for loading an LLM model, which is a time consuming task, and then providing it to a controller endpoint, where its already loaded, and fresh for use.

First, add a lifecycle hook in `src/app.rs`:

```rust
    // in src/app.rs, in your Hooks trait impl override the `after_routes` hook:

    async fn after_routes(router: axum::Router, _ctx: &AppContext) -> Result<axum::Router> {
        // cache should reside at: ~/.cache/huggingface/hub
        println!("loading model");
        let model = Llama::builder()
            .with_source(LlamaSource::llama_7b_code())
            .build()
            .unwrap();
        println!("model ready");
        let st = Arc::new(RwLock::new(model));

        Ok(router.layer(Extension(st)))
    }
```

Next, consume this state extension anywhere you like. Here's an example controller endpoint:

```rust
async fn candle_llm(Extension(m): Extension<Arc<RwLock<Llama>>>) -> impl IntoResponse {
    // use `m` from your state extension
    let prompt = "write binary search";
    ...
}
```

### Routes in Controllers

Controllers define Loco routes capabilities. In the example below, a controller creates one GET endpoint and one POST endpoint:

```rust
use axum::routing::{get, post};
Routes::new()
    .add("/", get(hello))
    .add("/echo", post(echo))
```

You can also define a `prefix` for all routes in a controller using the `prefix` function.

## Sending Responses

Response senders are in the `format` module. Here are a few ways to send responses from your routes:

```rust

// keep a best practice of returning a `Result<impl IntoResponse>` to be able to swap return types transparently
pub async fn list(...) -> Result<impl IntoResponse> // ..

// use `json`, `html` or `text` for simple responses
format::json(item)


// use `render` for a builder interface for more involved responses. you can still terminate with
// `json`, `html`, or `text`
format::render()
    .etag("foobar")?
    .json(Entity::find().all(&ctx.db).await?)
```

## Content type aware responses

You can opt-in into the responders mechanism, where a format type is detected
and handed to you.

Use the `Format` extractor for this:

```rust
pub async fn get_one(
    Format(respond_to): Format,
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let res = load_item(&ctx, id).await?;
    match respond_to {
        RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
        _ => format::json(item),
    }
}
```

## Content type aware responses and custom errors

Here is a case where you might want to both render differently based on
different formats AND ALSO, render differently based on kinds of errors you got.


```rust
pub async fn get_one(
    Format(respond_to): Format,
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    // having `load_item` is useful because inside the function you can call and use
    // '?' to bubble up errors, then, in here, we centralize handling of errors.
    // if you want to freely use code statements with no wrapping function, you can
    // use the experimental `try` feature in Rust where you can do:
    // ```
    // let res = try {
    //     ...
    //     ...
    // }
    //
    // match res { ..}
    // ```
    let res = load_item(&ctx, id).await;

    match res {
        // we're good, let's render the item based on content type
        Ok(item) => match respond_to {
            RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
            _ => format::json(item),
        },
        // we have an opinion how to render out validation errors, only in HTML content
        Err(Error::Model(ModelError::ModelValidation { errors })) => match respond_to {
            RespondTo::Html => {
                format::html(&format!("<html><body>errors: {errors:?}</body></html>"))
            }
            _ => bad_request("opaque message: cannot respond!"),
        },
        // we have no clue what this is, let the framework render default errors
        Err(err) => Err(err),
    }
}
```

Here, we also "centralize" our error handling by first wrapping the workflow in a function, and grabbing the result type.

Next we create a 2 level match to:

1. Match the result type
2. Match the format type

Where we lack the knowledge for handling, we just return the error as-is and let the framework render out default errors.

## Creating a Controller with the CLI Generator

Provides a convenient code generator to simplify the creation of a starter controller connected to your project. Additionally, a [test](@/docs/testing/controller.md) file is generated, enabling easy testing of your controller.

Generate a controller:

```sh
$ cargo loco generate controller [OPTIONS] <CONTROLLER_NAME>
```

After generating the controller, navigate to the created file in `src/controllers` to view the controller endpoints. You can also check the testing (in folder tests/requests) documentation for testing this controller.

## Creating a Controller Manually

#### 1. Create a Controller File

Start by creating a new file under the path `src/controllers`. For example, let's create a file named `example.rs`.

#### 2. Load the File in mod.rs

Ensure that you load the newly created controller file in the `mod.rs` file within the `src/controllers` folder.

#### 3. Register the Controller in App Hooks

In your App hook implementation (e.g., App struct), add your controller's `Routes` to `AppRoutes`:

```rust
// src/app.rs

pub struct App;
#[async_trait]
impl Hooks for App {
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes().prefix("prefix")
            .add_route(controllers::example::routes())
    }
    ...
}

```

## Displaying Registered Controllers

To view a list of all your registered controllers, execute the following command:

```sh
$ cargo loco controller

[GET] /_health
[GET] /_ping
[POST] /auth/forgot
[POST] /auth/login
[POST] /auth/register
[POST] /auth/reset
[POST] /auth/verify
[GET] /notes/
[POST] /notes/
[GET] /notes/:id
[DELETE] /notes/:id
[POST] /notes/:id
[GET] /user/current
```

This command will provide you with a comprehensive overview of the controllers currently registered in your system.

## Middleware

### Compression

`Loco` leverages [CompressionLayer](https://docs.rs/tower-http/0.5.0/tower_http/compression/index.html) to enable a `one click` solution.

To enable response compression, based on `accept-encoding` request header, simply edit the configuration as follows:

```yaml
#...
  middlewares:
    compression:
      enable: true
```

Doing so will compress each response and set `content-encoding` response header accordingly.


## Prcompressed assets

`Loco` leverages [ServeDir::precompressed_gzip](https://docs.rs/tower-http/latest/tower_http/services/struct.ServeDir.html#method.precompressed_gzip) to enable a `one click` solution of serving pre compressed assets.

If a static assets exists on the disk as a `.gz` file, `Loco` will serve it instead of compressing it on the fly.

```yaml
#...
middlewares:
  ...
  static_assets:
    ...
    precompressed: true
```

### (More middleware docs TBD)
