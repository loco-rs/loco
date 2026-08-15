---
title: Paginate query results
description: Page over entities with paginate/fetch_page, accept page/page_size from a request with PaginationQuery, and return a PageResponse from a controller.
sidebar:
  order: 3
---

**Goal:** return a page of rows plus paging metadata (`total_pages`, `total_items`, ...) from a controller endpoint, instead of loading an entire table.

This builds on [Query data with the condition DSL](/docs/how-to/query-data). For exact signatures and the `PagerMeta` shape, see [Query DSL & pagination](/docs/reference/query-pagination#pagination).

## 1. Accept pagination params in your request

`PaginationQuery` is a small, `#[serde(flatten)]`-friendly struct with two fields, `page` (1-based, default `1`) and `page_size` (default `25`). Flatten it into your controller's own query-params struct so callers can pass `?page=2&page_size=10` alongside your own filters:

```rust
use loco_rs::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListQueryParams {
    pub title: Option<String>,
    #[serde(flatten)]
    pub pagination: query::PaginationQuery,
}
```

## 2. Paginate an entity query with an optional condition

`query::paginate` takes a `Select<E>` (an unresolved entity query), an optional pre-built `Condition`, and a `&PaginationQuery`. It applies the condition for you, so don't call `.filter()` yourself first:

```rust
use axum::extract::{Query, State};

pub async fn list(
    State(ctx): State<AppContext>,
    Query(params): Query<ListQueryParams>,
) -> Result<Response> {
    let condition = params
        .title
        .as_ref()
        .map(|t| query::condition().contains(posts::Column::Title, t).build());

    let res = query::paginate(
        &ctx.db,
        posts::Entity::find(),
        condition,
        &params.pagination,
    )
    .await?;

    format::json(res)
}
```

## 3. Or paginate a pre-built selector with `fetch_page`

Use `fetch_page` when you've already composed a `Select` (with your own `.filter()`/`.order_by()`/joins) and just need to page over it — it takes no separate condition argument:

```rust
let selector = posts::Entity::find()
    .filter(query::condition().eq(posts::Column::UserId, user_id).build())
    .order_by_desc(posts::Column::CreatedAt);

let res = query::fetch_page(&ctx.db, selector, &query::PaginationQuery::page(2)).await?;
```

## 4. What you get back

Both functions return `LocoResult<PageResponse<T>>`:

```rust
pub struct PageResponse<T> {
    pub page: Vec<T>,
    pub meta: PagerMeta,
}

pub struct PagerMeta {
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
    pub total_items: u64,
}
```

Returning `res` from a controller (e.g. via `format::json(res)`) serializes to:

```json
{
  "page": [ { "id": 1, "title": "..." }, ... ],
  "meta": { "page": 2, "page_size": 25, "total_pages": 4, "total_items": 87 }
}
```

## 5. The scaffold's typed envelope

A generated `list` handler answers with `Page<T>` (`src/dtos/common.rs`), not `PageResponse` directly, because the typed React frontend needs a `ts-rs`-exported type and a flat body is easier to consume:

```json
{
  "items": [ { "id": 1, "title": "..." } ],
  "page": 2,
  "page_size": 25,
  "total_pages": 4,
  "total_items": 87
}
```

The metadata field names are `PagerMeta`'s, so there is one pagination vocabulary in the app whichever envelope a handler returns. Build it from a paginated query rather than by hand — that is what keeps the two in step:

```rust
let res = query::paginate(
    &ctx.db,
    posts::Entity::find().order_by_asc(posts::Column::Id),
    None,
    &pagination,
)
.await?;

Ok(Json(Page::from_query(res)))
```

`Page::from_query` maps each model through `T: From<M>` — the `impl From<Model> for PostDto` the scaffold already generates.

## Result

A request like `GET /posts?page=2&page_size=10&title=loco` returns exactly one page of matching rows plus enough metadata for a client to render "page 2 of 4" or build next/prev links — without loading the whole table or hand-writing `OFFSET`/`LIMIT` math. Remember `page` is 1-based on the way in; both functions handle the conversion to Sea-ORM's 0-based paging internally.

## Next

- [Query DSL & pagination reference](/docs/reference/query-pagination) for `PaginationQuery`'s exact defaults and the `paginate`/`fetch_page` signatures.
- [Query data](/docs/how-to/query-data) for building the `Condition` you pass in.
