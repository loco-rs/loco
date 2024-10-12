+++
title = "Controllers"
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
flair =[]
+++

`Loco` is a framework that wraps around [axum](https://crates.io/crates/axum), offering a straightforward approach to manage routes, middlewares, authentication, and more right out of the box. At any point, you can leverage the powerful axum Router and extend it with your custom middlewares and routes.

# Controllers and Routing


## Adding a controller

Provides a convenient code generator to simplify the creation of a starter controller connected to your project. Additionally, a test file is generated, enabling easy testing of your controller.

Generate a controller:

```sh
$ cargo loco generate controller [OPTIONS] <CONTROLLER_NAME>
```

After generating the controller, navigate to the created file in `src/controllers` to view the controller endpoints. You can also check the testing (in folder tests/requests) documentation for testing this controller.


### Displaying active routes

To view a list of all your registered controllers, execute the following command:

```sh
$ cargo loco routes

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


## Adding state

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

## Global app-wide state

Sometimes you might want state that can be shared between controllers, workers, and other areas of your app.

You can review the example [shared-global-state](https://github.com/loco-rs/shared-global-state) app to see how to integrate `libvips`, which is a C based image manipulation library. `libvips` requires an odd thing from the developer: to keep a single instance of it loaded per app process. We do this by keeping a [single `lazy_static` field](https://github.com/loco-rs/shared-global-state/blob/main/src/app.rs#L27-L34), and referring to it from different places in the app.

Read the following to see how it's done in each individual part of the app.

### Shared state in controllers

You can use the solution provided in this document. A live example [is here](https://github.com/loco-rs/loco/blob/master/examples/llm-candle-inference/src/app.rs#L41).

### Shared state in workers

Workers are intentionally verbatim initialized in [app hooks](https://github.com/loco-rs/loco/blob/master/starters/saas/src/app.rs#L59).

This means you can shape them as a "regular" Rust struct that takes a state as a field. Then refer to that field in perform.

[Here's how the worker is initialized](https://github.com/loco-rs/shared-global-state/blob/main/src/workers/downloader.rs#L19) with the global `vips` instance in the `shared-global-state` example.

Note that by-design _sharing state between controllers and workers have no meaning_, because even though you may choose to run workers in the same process as controllers initially (and share state) -- you'd want to quickly switch to proper workers backed by queue and running in a standalone workers process as you scale horizontally, and so workers should by-design have no shared state with controllers, for your own good.

### Shared state in tasks

Tasks don't really have a value for shared state, as they have a similar life as any exec'd binary. The process fires up, boots, creates all resources needed (connects to db, etc.), performs the task logic, and then the 

## Routes in Controllers

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

### Content type aware responses

You can opt-in into the responders mechanism, where a format type is detected
and handed to you.

Use the `Format` extractor for this:

```rust
pub async fn get_one(
    respond_to: RespondTo,
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

### Custom errors

Here is a case where you might want to both render differently based on
different formats AND ALSO, render differently based on kinds of errors you got.


```rust
pub async fn get_one(
    respond_to: RespondTo,
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

# Middleware

Loco comes with a set of built-in middleware out of the box. Some are enabled by default, while others need to be configured. Middleware registration is flexible and can be managed either through the `*.yaml` environment configuration or directly in the code.

You get all the enabled middlewares run the following command
<!-- <snip id="cli-middleware-list" inject_from="yaml" template="sh"> -->
```sh
cargo loco middleware --config
```
<!-- </snip> -->

### Authentication
In the `Loco` framework, middleware plays a crucial role in authentication. `Loco` supports various authentication methods, including JSON Web Token (JWT) and API Key authentication. This section outlines how to configure and use authentication middleware in your application.

#### JSON Web Token (JWT)

##### Configuration
By default, Loco uses Bearer authentication for JWT. However, you can customize this behavior in the configuration file under the auth.jwt section.
* *Bearer Authentication:* Keep the configuration blank or explicitly set it as follows:
  ```yaml
  # Authentication Configuration
  auth:
    # JWT authentication
    jwt:
      location: Bearer
  ...
  ```
* *Cookie Authentication:* Configure the location from which to extract the token and specify the cookie name:
  ```yaml
  # Authentication Configuration
  auth:
    # JWT authentication
    jwt:
      location: 
        from: Cookie
        name: token
  ...
  ```
* *Query Parameter Authentication:* Specify the location and name of the query parameter:
  ```yaml
  # Authentication Configuration
  auth:
    # JWT authentication
    jwt:
      location: 
        from: Query
        name: token
  ...
  ```

##### Usage
In your controller parameters, use `auth::JWT` for authentication. This triggers authentication validation based on the configured settings.
```rust
use loco_rs::prelude::*;

async fn current(
    auth: auth::JWT,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    // Your implementation here
}
```
Additionally, you can fetch the current user by replacing auth::JWT with `auth::ApiToken<users::Model>`.

#### API Key
For API Key authentication, use auth::ApiToken. This middleware validates the API key against the user database record and loads the corresponding user into the authentication parameter.
```rust
use loco_rs::prelude::*;

async fn current(
    auth: auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    // Your implementation here
}
```

## Catch Panic

This middleware catches panics that occur during request handling in the application. When a panic occurs, it logs the error and returns an internal server error response. This middleware helps ensure that the application can gracefully handle unexpected errors without crashing the server.

To disable the middleware edit the configuration as follows:

```yaml
#...
  middlewares:
    catch_panic:
      enable: false
```


## Limit Payload

Restricts the maximum allowed size for HTTP request payloads.
The middleware by default is enabled and configured to 2MB. 

You can disable or customize this behavior in your config file. You can set a few options:

```yaml
#...
  middlewares:
    limit_payload:
      enable: true
      body_limit: 5mb
```

##### Usage
In your controller parameters, use `axum::body::Bytes`.
```rust
use loco_rs::prelude::*;

async fn current(_body: axum::body::Bytes,) -> Result<Response> {
    // Your implementation here
}
```

## Timeout

Applies a timeout to requests processed by the application. The middleware ensures that requests do not run beyond the specified timeout period, improving the overall performance and responsiveness of the application.

If a request exceeds the specified timeout duration, the middleware will return a `408 Request Timeout` status code to the client, indicating that the request took too long to process.

To enable the middleware edit the configuration as follows:

```yaml
#...
  middlewares:
    timeout_request:
      enable: false
      timeout: 5000
```


## Logger

Provides logging functionality for HTTP requests. Detailed information about each request, such as the HTTP method, URI, version, user agent, and an associated request ID. Additionally, it integrates the application's runtime environment into the log context, allowing environment-specific logging (e.g., "development", "production").

To disable the middleware edit the configuration as follows:

```yaml
#...
  middlewares:
    logger:
      enable: false
```


## Fallback

When choosing the SaaS starter (or any starter that is not API-first), you get a default fallback behavior with the _Loco welcome screen_. This is a development-only mode where a `404` request shows you a nice and friendly page that tells you what happened and what to do next.


You can disable or customize this behavior in your `development.yaml` file. You can set a few options:


```yaml
# the default pre-baked welcome screen
fallback:
    enable: true
```

```yaml
# a different predefined 404 page
fallback:
    enable: true
    file: assets/404.html
```

```yaml
# a message, and customizing the status code to return 200 instead of 404
fallback:
    enable: true
    code: 200
    not_found: cannot find this resource
```

For production, it's recommended to disable this.

```yaml
# disable. you can also remove the `fallback` section entirely to disable
fallback:
    enable: false
```

## Remote IP

When your app is under a proxy or a load balancer (e.g. Nginx, ELB, etc.), it does not face the internet directly, which is why if you want to find out the connecting client IP, you'll get a socket which indicates an IP that is actually your load balancer instead.

The load balancer or proxy is responsible for doing the socket work against the real client IP, and then giving your app the load via the proxy back connection to your app.

This is why when your app has a concrete business need for getting the real client IP you need to use the de-facto standard proxies and load balancers use for handing you this information: the `X-Forwarded-For` header.

Loco provides the `remote_ip` section for configuring the `RemoteIP` middleware:

```yaml
server:
  middleware:
    # calculate remote IP based on `X-Forwarded-For` when behind a proxy or load balancer
    # use RemoteIP(..) extractor to get the remote IP.
    # without this middleware, you'll get the proxy IP instead.
    # For more: https://github.com/rails/rails/blob/main/actionpack/lib/action_dispatch/middleware/remote_ip.rb
    #
    # NOTE! only enable when under a proxy, otherwise this can lead to IP spoofing vulnerabilities
    # trust me, you'll know if you need this middleware.
    remote_ip:
      enable: true
      # # replace the default trusted proxies:
      # trusted_proxies:
      # - ip range 1
      # - ip range 2 ..
    # Generating a unique request ID and enhancing logging with additional information such as the start and completion of request processing, latency, status code, and other request details.
```

Then, use the `RemoteIP` extractor to get the IP:

```rust
#[debug_handler]
pub async fn list(ip: RemoteIP, State(ctx): State<AppContext>) -> Result<Response> {
    println!("remote ip {ip}");
    format::json(Entity::find().all(&ctx.db).await?)
}
```

When using the `RemoteIP` middleware, take note of the security implications vs. your current architecture (as noted in the documentation and in the configuration section): if your app is NOT under a proxy, you can be prone to IP spoofing vulnerability because anyone can set headers to arbitrary values, and specifically, anyone can set the `X-Forwarded-For` header.

This middleware is not enabled by default. Usually, you *will know* if you need this middleware and you will be aware of the security aspects of using it in the correct architecture. If you're not sure -- don't use it (keep `enable` to `false`).


## Secure Headers

Loco comes with default secure headers applied by the `secure_headers` middleware. This is similar to what is done in the Rails ecosystem with [secure_headers](https://github.com/github/secure_headers).

In your `server.middleware` YAML section you will find the `github` preset by default (which is what Github and Twitter recommend for secure headers).

```yaml
server:
  middleware:
    # set secure headers
    secure_headers:
      preset: github
```

You can also override select headers:

```yaml
server:
  middleware:
    # set secure headers
    secure_headers:
      preset: github
      overrides:
        foo: bar
```

Or start from scratch:

```yaml
server:
  middleware:
    # set secure headers
    secure_headers:
      preset: empty
      overrides:
        foo: bar
```

## Compression

`Loco` leverages [CompressionLayer](https://docs.rs/tower-http/0.5.0/tower_http/compression/index.html) to enable a `one click` solution.

To enable response compression, based on `accept-encoding` request header, simply edit the configuration as follows:

```yaml
#...
  middlewares:
    compression:
      enable: true
```

Doing so will compress each response and set `content-encoding` response header accordingly.

## Precompressed assets


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

## Handler and Route based middleware

`Loco` also allow us to apply [layers](https://docs.rs/tower/latest/tower/trait.Layer.html) to specific handlers or
routes.
For more information on handler and route based middleware, refer to the [middleware](/docs/the-app/middlewares)
documentation.

## Cors
This middleware enables Cross-Origin Resource Sharing (CORS) by allowing configurable origins, methods, and headers in HTTP requests. 
It can be tailored to fit various application requirements, supporting permissive CORS or specific rules as defined in the middleware configuration.

```yaml
#...
middlewares:
  ...
  cors:
    enable: true
    # Set the value of the [`Access-Control-Allow-Origin`][mdn] header
    # allow_origins:
    #   - https://loco.rs
    # Set the value of the [`Access-Control-Allow-Headers`][mdn] header
    # allow_headers:
    # - Content-Type
    # Set the value of the [`Access-Control-Allow-Methods`][mdn] header
    # allow_methods:
    #   - POST
    # Set the value of the [`Access-Control-Max-Age`][mdn] header in seconds
    # max_age: 3600

```

### Handler based middleware:

Apply a layer to a specific handler using `layer` method.

```rust
// src/controllers/auth.rs
pub fn routes() -> Routes {
    Routes::new()
        .prefix("auth")
        .add("/register", post(register).layer(middlewares::log::LogLayer::new()))
}
```

### Route based middleware:

Apply a layer to a specific route using `layer` method.

```rust
// src/main.rs
pub struct App;

#[async_trait]
impl Hooks for App {
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(
                controllers::auth::routes()
                    .layer(middlewares::log::LogLayer::new()),
            )
    }
}
```

# Pagination

In many scenarios, when querying data and returning responses to users, pagination is crucial. In `Loco`, we provide a straightforward method to paginate your data and maintain a consistent pagination response schema for your API responses.

## Using pagination

```rust
use loco_rs::prelude::*;

let res = query::fetch_page(&ctx.db, notes::Entity::find(), &query::PaginationQuery::page(2)).await;
```


## Using pagination With Filter
```rust
use loco_rs::prelude::*;

let pagination_query = query::PaginationQuery {
    page_size: 100,
    page: 1,
};

let condition = query::condition().contains(notes::Column::Title, "loco");
let paginated_notes = query::paginate(
    &ctx.db,
    notes::Entity::find(),
    Some(condition.build()),
    &pagination_query,
)
.await?;
```

- Start by defining the entity you want to retrieve.
- Create your query condition (in this case, filtering rows that contain "loco" in the title column).
- Define the pagination parameters.
- Call the paginate function.

### Pagination view
After creating getting the `paginated_notes` in the previous example, you can choose which fields from the model you want to return and keep the same pagination response in all your different data responses.

Define the data you're returning to the user in Loco views. If you're not familiar with views, refer to the [documentation](@/docs/the-app/views.md) for more context.


Create a notes view file in `src/view/notes` with the following code:

```rust
use loco_rs::{
    controller::views::pagination::{Pager, PagerMeta},
    prelude::model::query::PaginatedResponse,
};
use serde::{Deserialize, Serialize};

use crate::models::_entities::notes;

#[derive(Debug, Deserialize, Serialize)]
pub struct ListResponse {
    id: i32,
    title: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaginationResponse {}

impl From<notes::Model> for ListResponse {
    fn from(note: notes::Model) -> Self {
        Self {
            id: note.id.clone(),
            title: note.title.clone(),
            content: note.content,
        }
    }
}

impl PaginationResponse {
    #[must_use]
    pub fn response(data: PaginatedResponse<notes::Model>, pagination_query: &PaginationQuery) -> Pager<Vec<ListResponse>> {
        Pager {
            results: data
                .page
                .into_iter()
                .map(ListResponse::from)
                .collect::<Vec<ListResponse>>(),
            info: PagerMeta {
                page: pagination_query.page,
                page_size: pagination_query.page_size,
                total_pages: data.total_pages,
            },
        }
    }
}
```


# Testing 
When testing controllers, the goal is to call the router's controller endpoint and verify the HTTP response, including the status code, response content, headers, and more.

To initialize a test request, use `testing::request`, which prepares your app routers, providing the request instance and the application context.

In the following example, we have a POST endpoint that returns the data sent in the POST request.

```rust

#[tokio::test]
#[serial]
async fn can_print_echo() {
    configure_insta!();

    testing::request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/example")
            .json(&serde_json::json!({"site": "Loco"}))
            .await;

        assert_debug_snapshot!((response.status_code(), response.text()));
    })
    .await;
}
```

As you can see initialize the testing request and using `request` instance calling /example endpoing.
the request returns a `Response` instance with the status code and the response test
