+++
title = "Query DSL & pagination"
description = "The ConditionBuilder fluent filter DSL, date-range helper, pagination API, and the model-layer error/Authenticable types."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 10
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

`loco_rs::model::query` (reachable as `loco_rs::prelude::query` or `loco_rs::prelude::model::query`) is a small fluent DSL over Sea-ORM's `Condition`, plus pagination helpers that wrap Sea-ORM's `PaginatorTrait`. This page documents `ConditionBuilder`'s full operator surface, `DateRangeBuilder`, `SortDirection`, the pagination types/functions, and the model-layer `ModelError`/`ModelResult`/`Authenticable` types the query and model code return.

All of this module is gated behind the `with-db` feature (`Cargo.toml:44-49` pulls `sea-orm`/`sea-orm-migration`/`sqlx`); `prelude.rs:29-30` and `prelude.rs:52-55` gate `query`, `ModelError`, `ModelResult`, `Authenticable` on the same feature.

## `ConditionBuilder` — fluent filter DSL

Module: `src/model/query/dsl/mod.rs`, re-exported at `src/model/query/mod.rs:4`.

```rust
pub struct ConditionBuilder {
    condition: Condition, // sea_orm::Condition
}
```

Two entry points build a `ConditionBuilder`:

| Fn | Signature | Behavior | Anchor |
|---|---|---|---|
| `condition()` | `condition() -> ConditionBuilder` | Starts from `Condition::all()` (AND-combined). | `dsl/mod.rs:38` |
| `with(condition)` | `const fn with(condition: Condition) -> ConditionBuilder` | Wraps an existing Sea-ORM `Condition` (used internally to chain builder calls). | `dsl/mod.rs:45` |

`ConditionBuilder` implements `From<ConditionBuilder> for Condition` (`dsl/mod.rs:167`), and every builder method below returns `Self` (consuming `self`) so calls chain; `.build()` finalizes to a `sea_orm::Condition`:

```rust
pub fn build(&self) -> Condition  // dsl/mod.rs:701
```

### Operators

Every operator exists twice: as a **free function** `query::<op>(col, ..)` that starts a new builder (shorthand for `condition().<op>(..)`), and as a **method** on `ConditionBuilder` for chaining. Both forms accept a Sea-ORM `ColumnTrait` (an entity's generated `Column` enum) as the column argument.

| Operator | Free fn (anchor) | Builder method (anchor) | Args | SQL produced |
|---|---|---|---|---|
| Equals | `eq` `dsl/mod.rs:51` | `eq` `:235` | `col: T, value: V: Into<Value>` | `col = value` |
| Not equals | `not_equal` `:57` | `ne` `:260` | `col, value` | `col <> value` |
| Greater than | `gt` `:63` | `gt` `:285` | `col, value` | `col > value` |
| Greater than or equal | `gt_equal` `:69` | `gte` `:311` | `col, value` | `col >= value` |
| Less than | `lt` `:75` | `lt` `:337` | `col, value` | `col < value` |
| Less than or equal | `lt_equal` `:81` | `lte` `:363` | `col, value` | `col <= value` |
| Between | `between` `:87` | `between` `:389` | `col, a: V, b: V` | `col BETWEEN a AND b` |
| Not between | `not_between` `:93` | `not_between` `:415` | `col, a, b` | `col NOT BETWEEN a AND b` |
| Like | `like` `:99` | `like` `:441` | `col, pattern: V: Into<String>` | `col LIKE pattern` (caller supplies wildcards) |
| Not like | `not_like` `:105` | `not_like` `:467` | `col, pattern` | `col NOT LIKE pattern` |
| Starts with | `starts_with` `:111` | `starts_with` `:493` | `col, s: V: Into<String>` | `col LIKE 's%'` |
| Ends with | `ends_with` `:117` | `ends_with` `:519` | `col, s` | `col LIKE '%s'` |
| Contains | `contains` `:123` | `contains` `:545` | `col, s` | `col LIKE '%s%'` |
| Is null | `is_null` `:130` | `is_null` `:572` | `col` | `col IS NULL` |
| Is not null | `is_not_null` `:137` | `is_not_null` `:599` | `col` | `col IS NOT NULL` |
| Is in | `is_in` `:144` | `is_in` `:626` | `col, values: I: IntoIterator<Item = V>` | `col IN (values...)` |
| Is not in | `is_not_in` `:154` | `is_not_in` `:657` | `col, values` | `col NOT IN (values...)` |
| Date range | `date_range` `:163` | `date_range` `:696` | `col` — returns a `DateRangeBuilder<T>`, not `Self` | see [Date range](#daterangebuilder-date-range-filtering) below |

That is 17 comparison/pattern/membership operators plus `date_range`, matching the ~18-operator surface of the module.

Example (from the module's own doctest, `dsl/mod.rs:194-233`):

```rust
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, QueryFilter};

let cond = query::condition().eq(test_db::Column::Id, 1).build();
test_db::Entity::find().filter(cond);
// WHERE "loco"."id" = 1
```

`like`/`not_like` pass the pattern through verbatim (the caller writes `%` wildcards); `starts_with`/`ends_with`/`contains` add the wildcard(s) for you and always compile down to `LIKE`.

### `DateRangeBuilder` — date-range filtering

`date_range(col)` (either the free fn or the `ConditionBuilder` method) returns a `DateRangeBuilder<T>` instead of `Self`, because a date range needs 0, 1, or 2 bounds before it can become a condition. Struct and impl: `src/model/query/dsl/date_range.rs:7-66`.

| Method | Signature | Anchor |
|---|---|---|
| `new` | `const fn new(condition_builder: ConditionBuilder, col: T) -> Self` | `:15` |
| `dates` | `fn dates(self, from: Option<&NaiveDateTime>, to: Option<&NaiveDateTime>) -> Self` | `:25` |
| `from` | `fn from(self, from: &NaiveDateTime) -> Self` | `:35` |
| `to` | `fn to(self, to: &NaiveDateTime) -> Self` | `:45` |
| `build` | `fn build(self) -> ConditionBuilder` | `:54` |

**Boundary behavior is asymmetric** (`date_range.rs:55-63`) — this is the one non-obvious semantic in the DSL, worth knowing before using it:

| Bounds set | SQL |
|---|---|
| neither `from` nor `to` | no condition added (passthrough) |
| `to` only | `col < to` (strict) |
| `from` only | `col > from` (strict) |
| both `from` and `to` | `col BETWEEN from AND to` (inclusive) |

So a single-ended range is exclusive at the bound, but a double-ended range is inclusive at both bounds — `date_range(col).from(&d).build()` will *not* include rows exactly at `d`, but `date_range(col).dates(Some(&d), Some(&d2)).build()` *will* include rows exactly at `d` or `d2`.

### `SortDirection`

`src/model/query/dsl/mod.rs:17-35`:

```rust
pub enum SortDirection {
    Desc, // serde "desc"
    Asc,  // serde "asc"
}
```

- Derives `Deserialize, Serialize` with `#[serde(rename = "desc"/"asc")]` on each variant — meant to deserialize directly from a query-string sort param.
- `order(&self) -> Order` (`:29`, `#[must_use] const fn`) converts to `sea_orm::sea_query::Order::Desc`/`Order::Asc` for use with `.order_by(col, direction.order())`.

This enum is not itself a `ConditionBuilder` operator — it pairs with Sea-ORM's own `QueryOrder::order_by`, orthogonal to filtering.

## Pagination

Module: `src/model/query/paginate/mod.rs`, re-exported at `src/model/query/mod.rs:5`. Reached as `query::paginate`, `query::fetch_page`, `query::PaginationQuery`.

### `PaginationQuery`

```rust
pub struct PaginationQuery {
    pub page_size: u64, // default 25
    pub page: u64,      // default 1, 1-based
}
```
(`paginate/mod.rs:31-45`)

| Field | Type | Default | Notes |
|---|---|---|---|
| `page_size` | `u64` | `25` (`default_page_size`, `:5-7`) | Rows per page. |
| `page` | `u64` | `1` (`default_page`, `:9-11`) | **1-based.** `paginate`/`fetch_page` internally `saturating_sub(1)` to reach Sea-ORM's 0-based `fetch_page`. |

- Both fields use a custom `deserialize_pagination_filter` (`:69-75`) that parses a **string** into `u64` — a workaround for a `serde_urlencoded` bug where numeric query-string params don't deserialize directly to integers. This makes `PaginationQuery` safe to use as a `#[serde(flatten)]` field inside an axum `Query<T>` extractor struct, e.g.:

  ```rust
  #[derive(Debug, Deserialize)]
  pub struct ListQueryParams {
      pub title: Option<String>,
      pub content: Option<String>,
      #[serde(flatten)]
      pub pagination: query::PaginationQuery,
  }
  ```
  (doctest at `paginate/mod.rs:19-30`)

- `PaginationQuery::page(page: u64) -> Self` (`:49`) — constructs with the given page and `page_size` defaulted via `..Default::default()`.
- `impl Default for PaginationQuery` (`:58-65`) — `page_size = 25`, `page = 1`.

### `PageResponse<T>` and `PagerMeta`

```rust
pub struct PageResponse<T> {
    pub page: Vec<T>,
    pub meta: PagerMeta,
}
```
(`paginate/mod.rs:80-83`; `PagerMeta` is `crate::controller::views::pagination::PagerMeta`, imported at `:77`)

`PagerMeta` (`src/controller/views/pagination.rs:13-22`) — not re-derived by the inventory, verified directly from source for this page:

```rust
pub struct PagerMeta {
    pub page: u64,       // serializes as "page"
    pub page_size: u64,  // serializes as "page_size"
    pub total_pages: u64,// serializes as "total_pages"
    pub total_items: u64,// serializes as "total_items"
}
```

### `paginate` and `fetch_page`

| Fn | Signature | Anchor |
|---|---|---|
| `paginate` | `async fn paginate<E>(db: &DatabaseConnection, entity: Select<E>, condition: Option<Condition>, pagination_query: &PaginationQuery) -> LocoResult<PageResponse<E::Model>> where E: EntityTrait, E::Model: Sync` | `paginate/mod.rs:146` |
| `fetch_page` | `async fn fetch_page<'db, C, S>(db: &'db C, selector: S, pagination_query: &PaginationQuery) -> LocoResult<PageResponse<...>> where C: ConnectionTrait + Sync, S: PaginatorTrait<'db, C> + Send` | `paginate/mod.rs:204` |

Both:
- Take the caller's 1-based `pagination_query.page` and internally do `.saturating_sub(1)` (`:156`, `:213`) before calling Sea-ORM's `Paginator::fetch_page` (which is 0-based).
- Call `query.num_items_and_pages().await?` to populate `PagerMeta.total_pages`/`total_items`, then `query.fetch_page(page).await?` for the row data.
- Return `Ok(PageResponse { page, meta })` — the crate's `LocoResult<T>` (i.e. `crate::Result<T, crate::errors::Error>`).

`paginate` takes a `Select<E>` (an entity query builder) plus an optional pre-built `Condition` — it applies `.filter(condition)` itself if one is given, so you don't chain `.filter()` before calling it. `fetch_page` is the more generic form: it accepts anything implementing Sea-ORM's `PaginatorTrait` directly (so you can pre-build arbitrary selects, including `.order_by(..)`, and just page over the result), but does not take a separate `Condition` argument — filter and ordering must already be applied to the selector you pass in.

```rust
// paginate: entity + optional condition + pagination query
let condition = query::condition().contains(Column::Name, "loco").build();
let res = query::paginate(&db, Entity::find(), Some(condition), &pagination_query).await;

// fetch_page: pre-built selector (any PaginatorTrait), no separate condition arg
let res = query::fetch_page(&db, Entity::find(), &query::PaginationQuery::page(2)).await;
```
(adapted from doctests at `paginate/mod.rs:92-140` and `:185-197`)

## Model-layer error types

`src/model/mod.rs` — the error type returned by model/authn code (distinct from the crate-wide `loco_rs::errors::Error`; see the [error model reference](@/docs/reference/errors.md)).

### `ModelError` / `ModelResult`

```rust
pub enum ModelError {
    EntityAlreadyExists,
    EntityNotFound,
    Validation(ModelValidationErrors),      // #[from]
    #[cfg(feature = "auth")]
    Jwt(jsonwebtoken::errors::Error),        // #[from]
    DbErr(sea_orm::DbErr),                   // #[from]
    Any(Box<dyn std::error::Error + Send + Sync>), // #[from]
    Message(String),
}

pub type ModelResult<T, E = ModelError> = std::result::Result<T, E>;
```
(`src/model/mod.rs:13-35` for the enum, `:38` for the alias)

Note: `ModelError` is **not** `#[non_exhaustive]` (unlike the crate-wide `Error`) — this is the model layer's own, smaller error enum, not the one documented on the [error model reference](@/docs/reference/errors.md) page. `Jwt` only exists when the `auth` feature is enabled (`mod.rs:23-25`).

Constructors:

| Fn | Signature | Anchor |
|---|---|---|
| `ModelError::wrap` | `#[must_use] fn wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self` — builds `Any(Box::new(err))` | `:42-44` |
| `ModelError::to_msg` | `#[must_use] fn to_msg(err: impl std::error::Error + Send + Sync + 'static) -> Self` — builds `Message(err.to_string())` | `:47-49` |
| `ModelError::msg` | `#[must_use] fn msg(s: &str) -> Self` — builds `Message(s.to_string())` | `:52-54` |

`loco_rs::errors::Error` itself has a `Model(#[from] crate::model::ModelError)` variant (`with-db`-gated), so a `ModelError` returned from a model method converts automatically into the crate-wide `Error` at a controller boundary via `?`.

### `Authenticable`

```rust
#[async_trait]
pub trait Authenticable: Clone {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self>;
    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self>;
}
```
(`src/model/mod.rs:56-60`)

A user model (typically the `users` entity) implements `Authenticable` so the auth extractors (`JWT`, `JWTWithUser`, `ApiToken` under `prelude::auth`, feature `auth`) can look the caller up: `find_by_claims_key` resolves a JWT's claims subject to a model instance; `find_by_api_key` resolves a bearer/API-key header the same way. Both are `async` (the trait itself is `#[async_trait]`) and return `ModelResult<Self>`, so a lookup failure surfaces as a `ModelError` (typically `EntityNotFound` or `DbErr`).

All four items (`query`, `ModelError`, `ModelResult`, `Authenticable`) are re-exported from `loco_rs::prelude` (`prelude.rs:29-30`, `with-db`-gated).
