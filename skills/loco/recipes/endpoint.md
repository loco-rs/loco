# Recipe: controllers, routes, and responses

```sh
cargo loco generate controller posts --api     # JSON API
cargo loco generate controller posts           # HTML views
cargo loco generate scaffold posts title:string! --api   # model + migration + controller + views + tests
```

The generator writes the file **and** registers it in `src/app.rs`. Rust has no
autoloading — hand-wiring a route is how "the handler exists but 404s" happens.

## Shape of a controller

```rust
use loco_rs::prelude::*;

use crate::{models::posts, views::post::PostResponse};

#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let posts = posts::Model::published(&ctx.db).await?;
    format::json(posts.iter().map(PostResponse::from).collect::<Vec<_>>())
}

#[debug_handler]
async fn show(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {
    let post = posts::Model::find_by_id(&ctx.db, id).await?;
    format::json(PostResponse::from(&post))
}

#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    Json(params): Json<CreateParams>,
) -> Result<Response> {
    let post = posts::Model::create(&ctx.db, &params).await?;
    format::json(PostResponse::from(&post))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/posts")
        .add("/", get(list))
        .add("/", post(create))
        .add("/{id}", get(show))
}
```

`#[debug_handler]` is not optional in practice — without it, a handler that
fails Axum's trait bounds produces an unreadable error.

`State`, `Path`, `Json`, `Query`, `Form`, `Multipart`, `get`, `post`, `put`,
`patch`, `delete`, `format`, `bad_request`, `unauthorized`, `not_found` all come
from `loco_rs::prelude::*`. Do not import them from `axum::` — if you are
reaching into `axum::` in a controller, check the prelude list in
`api-index.md` first.

## Building routes

`Routes` is a builder. Two equivalent styles:

```rust
let explicit = Routes::new().prefix("/api/posts").add("/", get(list));
let convenience = Routes::new().prefix("/api/posts").get("/", list);
```

Also available: `Routes::at(prefix)`, `.nest(path, routes)`, `.merge(other)`,
`.layer(l)` for a route-group-scoped tower layer.

Path parameters use Axum 0.8 brace syntax: `/{id}`, not `/:id`.

## Responses — never serialize an entity

`format::json(user)` on a `users::Model` puts the password hash and API key on
the wire. Render a view type from `src/views/`:

```rust
// src/views/post.rs
use serde::{Deserialize, Serialize};
use crate::models::_entities::posts;

#[derive(Debug, Deserialize, Serialize)]
pub struct PostResponse {
    pub id: i32,
    pub title: String,
}

impl From<&posts::Model> for PostResponse {
    fn from(post: &posts::Model) -> Self {
        Self { id: post.id, title: post.title.clone() }
    }
}
```

### Paginated lists

Loco owns the wire shape for a page. Do not write your own envelope — the
struct with `page`, `page_size`, `total_pages`, `total_items` already exists,
and a hand-rolled one silently changes the response body clients expect.

`PaginationQuery` deserializes straight from the query string with defaults, so
it is an extractor:

```rust
#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    Query(pagination): Query<query::PaginationQuery>,
) -> Result<Response> {
    let res = query::paginate(&ctx.db, posts::Entity::find(), None, &pagination).await?;
    let results: Vec<PostResponse> = res.page.iter().map(PostResponse::from).collect();
    format::json(Pager::new(results, res.meta))
}
```

`paginate` returns `PageResponse { page: Vec<Model>, meta: PagerMeta }`, and
`res.meta` is already the `PagerMeta` that `Pager` wants — map the rows to your
view type and hand the meta straight through. The body is:

```json
{ "results": [ ... ], "pagination": { "page": 1, "page_size": 25, "total_pages": 4, "total_items": 87 } }
```

Note it serializes as `pagination`, not `info`. Pass a condition as the third
argument to filter: `query::paginate(&ctx.db, E::find(), Some(cond), &pagination)`.

### The `format` helpers

Each returns `Result<Response>`, so a handler ends with one of these and no
`Ok(...)` wrapper:

| Helper | Signature |
|---|---|
| `format::json(t)` | any `T: Serialize` |
| `format::empty()` | 200, no body |
| `format::empty_json()` | `{}` |
| `format::text(&str)` | `text/plain` |
| `format::html(&str)` | `text/html` |
| `format::yaml(&str)` | `application/yaml` |
| `format::redirect(to: &str)` | a redirect response |
| `format::view(&v, key, data)` | render through the view engine |
| `format::template(name, data)` | render a template by name |

```rust
#[debug_handler]
async fn go(State(ctx): State<AppContext>, Path(slug): Path<String>) -> Result<Response> {
    let link = links::Model::find_by_slug(&ctx, &slug).await?;
    format::redirect(&link.url)
}
```

### When you need a status, header, cookie, or etag

`format::render()` is the builder. Chain the modifiers, then finish with the
same body helpers:

```rust
format::render()
    .status(StatusCode::CREATED)
    .header("x-request-id", &id)
    .json(LinkResponse::from(&link))
```

Available on the builder: `.status()`, `.header()`, `.etag()`, `.cookies()`,
then a terminal `.json()`, `.text()`, `.html()`, `.empty()`, `.view()`,
`.template()`, `.redirect()`, or `.redirect_with_header_key()`.

Reach for `render()` only when you need one of those; a plain `format::json(..)`
is the normal case.

## Errors

Handlers return `loco_rs::Result<Response>`. Use `?`. A `ModelError` converts
into `Error` automatically, so a model call is just `.await?`.

Return typed errors so the right status comes out:

```rust
return bad_request("title is required");
return unauthorized("bad token");
return not_found();
```

`Error::string("...")` produces a **500**. Using it for a client error is a bug.
Never `.unwrap()` or `.expect()` in a handler — a panic is a dropped connection,
not a 500.

### What a bare `?` on a model call already answers

`ModelError` has its own status mapping, so most error handling is *not writing
any*. Know what you get before you add a `match`:

| What the model returns | Status | Body |
|---|---|---|
| `ModelError::EntityNotFound` | **404** | `{"error": "not_found", ...}` |
| `ModelError::EntityAlreadyExists` | **409** | `{"error": "conflict", ...}` |
| `ModelError::Validation(..)` | **400** | the per-field validation errors |
| `ModelError::DbErr`, `Jwt`, `Any`, `Message` | **500** | generic; details are not leaked |

```rust
// The 404 is already correct — there is nothing to add.
let user = users::Model::find_by_pid(&ctx.db, &pid).await?;
```

**409, not 400, is the default for a duplicate.** If your API contract says a
conflict must be 400, you have to say so explicitly — `?` will not do it:

```rust
match users::Model::find_by_pid(&ctx.db, &pid).await {
    Err(ModelError::EntityAlreadyExists) => bad_request("that slug is taken"),
    other => other?,
};
```

Match a specific variant only when you need a status or message different from
the table. Restating the mapping the framework already applies is the most
common form of redundant error handling in a Loco handler.

## Params

Define a params struct, derive `Deserialize`, and extract it. Validation belongs
on the model (`Validatable`), not as inline `if` checks in the handler.

```rust
#[derive(Debug, Deserialize)]
pub struct CreateParams {
    pub title: String,
    pub content: String,
}
```

## Check your work

```sh
cargo loco routes      # every route actually registered
```

If your handler is not listed, it was never wired into `src/app.rs`.
