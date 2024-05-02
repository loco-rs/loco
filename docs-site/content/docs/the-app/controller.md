+++
title = "Controllers"
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
flair = []
+++

`Loco` is a framework that wraps around [axum](https://crates.io/crates/axum), offering a straightforward approach to
manage routes, middlewares, authentication, and more right out of the box. At any point, you can leverage the powerful
axum Router and extend it with your custom middlewares and routes.

# Controllers and Routing

## Adding a controller

Provides a convenient code generator to simplify the creation of a starter controller connected to your project.
Additionally, a test file is generated, enabling easy testing of your controller.

Generate a controller:

```sh
$ cargo loco generate controller [OPTIONS] <CONTROLLER_NAME>
```

After generating the controller, navigate to the created file in `src/controllers` to view the controller endpoints. You
can also check the testing (in folder tests/requests) documentation for testing this controller.

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

Your app context and state is held in `AppContext` and is what Loco provides and sets up for you. There are cases where
you'd want to load custom data,
logic, or entities when the app starts and be available to use in all controllers.

You could do that by using Axum's `Extension`. Here's an example for loading an LLM model, which is a time consuming
task, and then providing it to a controller endpoint, where its already loaded, and fresh for use.

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

## Routes in Controllers

Controllers define Loco routes capabilities. In the example below, a controller creates one GET endpoint and one POST
endpoint:

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
.etag("foobar") ?
.json(Entity::find().all( & ctx.db).await?)
```

### Content type aware responses

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

### Custom errors

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

Here, we also "centralize" our error handling by first wrapping the workflow in a function, and grabbing the result
type.

Next we create a 2 level match to:

1. Match the result type
2. Match the format type

Where we lack the knowledge for handling, we just return the error as-is and let the framework render out default
errors.

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

### Authentication

In the `Loco` framework, middleware plays a crucial role in authentication. `Loco` supports various authentication
methods, including JSON Web Token (JWT) and API Key authentication. This section outlines how to configure and use
authentication middleware in your application.

#### JSON Web Token (JWT)

##### Configuration

By default, Loco uses Bearer authentication for JWT. However, you can customize this behavior in the configuration file
under the auth.jwt section.

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

In your controller parameters, use `auth::JWT` for authentication. This triggers authentication validation based on the
configured settings.

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

For API Key authentication, use auth::ApiToken. This middleware validates the API key against the user database record
and loads the corresponding user into the authentication parameter.

```rust
use loco_rs::prelude::*;

async fn current(
    auth: auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    // Your implementation here
}
```

## Compression

`Loco` leverages [CompressionLayer](https://docs.rs/tower-http/0.5.0/tower_http/compression/index.html) to enable
a `one click` solution.

To enable response compression, based on `accept-encoding` request header, simply edit the configuration as follows:

```yaml
#...
middlewares:
  compression:
    enable: true
```

Doing so will compress each response and set `content-encoding` response header accordingly.

## Precompressed assets

`Loco`
leverages [ServeDir::precompressed_gzip](https://docs.rs/tower-http/latest/tower_http/services/struct.ServeDir.html#method.precompressed_gzip)
to enable a `one click` solution of serving pre compressed assets.

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

`Loco` also allow us to apply `[layers](https://docs.rs/tower/latest/tower/trait.Layer.html) to specific routes. Here's
an example of how to apply a layer to a specific route:
For more information on handler and route based middleware, refer to the [middleware](/docs/the-app/middlewares)
documentation.

### Handler based middleware:

```rust
// src/controllers/auth.rs
pub fn routes() -> Routes {
    Routes::new()
        .prefix("auth")
        .add("/register", post(register).layer(middlewares::log::LogLayer::new()))
}
```

### Route based middleware:

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

In many scenarios, when querying data and returning responses to users, pagination is crucial. In `Loco`, we provide a
straightforward method to paginate your data and maintain a consistent pagination response schema for your API
responses.

## Using pagination

```rust
use loco_rs::prelude::*;

let pagination_query = model::query::PaginationQuery {
page_size: 100,
page: 1,
};

let condition = model::query::dsl::condition().contains(notes::Column::Title, "loco");
let paginated_notes = model::query::exec::paginate(
& ctx.db,
notes::Entity::find(),
Some(condition.build()),
& pagination_query,
)
.await?;
```

- Start by defining the entity you want to retrieve.
- Create your query condition (in this case, filtering rows that contain "loco" in the title column).
- Define the pagination parameters.
- Call the paginate function.

### Pagination view

After creating getting the `paginated_notes` in the previous example, you can choose which fileds from the model you
want to return and keep the same pagination response in all your different data responses.

Define the data you're returning to the user in Loco views. If you're not familiar with views, refer to
the [documentation]((@/docs/the-app/views.md)) for more context.

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
    pub fn response(data: PaginatedResponse<notes::Model>) -> Pager<Vec<ListResponse>> {
        Pager {
            results: data
                .rows
                .into_iter()
                .map(ListResponse::from)
                .collect::<Vec<ListResponse>>(),
            info: PagerMeta {
                page: data.info.page,
                page_size: data.info.page_size,
                total_pages: data.info.total_pages,
            },
        }
    }
}
```

# Testing

When testing controllers, the goal is to call the router's controller endpoint and verify the HTTP response, including
the status code, response content, headers, and more.

To initialize a test request, use `testing::request`, which prepares your app routers, providing the request instance
and the application context.

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
