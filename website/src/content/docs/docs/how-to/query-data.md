---
title: Query data with the condition DSL
description: Filter entities with ConditionBuilder operators, build date ranges, and sort results — without hand-writing Sea-ORM conditions.
sidebar:
  order: 2
---

**Goal:** filter rows from a model using Loco's `ConditionBuilder` fluent DSL instead of hand-assembling a Sea-ORM `Condition`.

This assumes a working model (see [Add a model](/docs/how-to/add-model)). For the exhaustive operator list, exact signatures, and the `date_range` boundary semantics, see [Query DSL & pagination](/docs/reference/query-pagination).

## 1. Import the DSL

`query` is the model-layer query module, reachable straight from the prelude:

```rust
use loco_rs::prelude::*;
use sea_orm::EntityTrait;
```

`query::condition()` starts a builder; `.build()` finalizes it into a `sea_orm::Condition` you pass to `.filter(..)`.

## 2. Build a simple filter

```rust
let cond = query::condition()
    .eq(users::Column::Email, "user1@example.com")
    .build();

let user = users::Entity::find().filter(cond).one(&db).await?;
```

This is the same pattern the demo app's `users` model uses for all of its lookups, e.g. `Model::find_by_email` in `examples/demo/src/models/users.rs`:

```rust
pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
    let user = users::Entity::find()
        .filter(
            model::query::condition()
                .eq(users::Column::Email, email)
                .build(),
        )
        .one(db)
        .await?;
    user.ok_or_else(|| ModelError::EntityNotFound)
}
```

## 3. Chain multiple conditions

Every operator returns `Self`, so conditions chain and AND together by default:

```rust
let cond = query::condition()
    .contains(posts::Column::Title, "loco")
    .gt(posts::Column::Views, 10)
    .is_not_null(posts::Column::PublishedAt)
    .build();

let published = posts::Entity::find().filter(cond).all(&db).await?;
```

Available operators (see the [reference](/docs/reference/query-pagination#operators) for the full table): `eq`/`ne`, `gt`/`gte`/`lt`/`lte`, `between`/`not_between`, `like`/`not_like`, `starts_with`/`ends_with`/`contains`, `is_null`/`is_not_null`, `is_in`/`is_not_in`, and `date_range`. Each also exists as a free function (`query::eq(col, v)`) that starts a new builder — useful when you only need one condition.

## 4. Filter a date range

`date_range` returns a `DateRangeBuilder` instead of `Self`, because a range can have zero, one, or two bounds:

```rust
use chrono::NaiveDateTime;

let from: NaiveDateTime = /* ... */;
let to: NaiveDateTime = /* ... */;

let cond = query::condition()
    .date_range(posts::Column::CreatedAt)
    .dates(Some(&from), Some(&to))
    .build() // DateRangeBuilder -> ConditionBuilder
    .build(); // ConditionBuilder -> Condition
```

<div class="infobox">
Boundary behavior is asymmetric: a single-ended range (only <code>from</code> or only <code>to</code>) is <b>strict</b> (<code>&gt;</code> / <code>&lt;</code>), but a double-ended range (both <code>from</code> and <code>to</code>) is <b>inclusive</b> (<code>BETWEEN</code>). See <a href="/docs/reference/query-pagination#daterangebuilder-date-range-filtering">the reference</a> for the full table.
</div>

## 5. Sort results

`SortDirection` is a small serde-friendly enum that converts to Sea-ORM's `Order` — it's orthogonal to `ConditionBuilder`, meant to pair with `.order_by()`:

```rust
use loco_rs::model::query::SortDirection;

let direction = SortDirection::Desc;

let recent = posts::Entity::find()
    .filter(cond)
    .order_by(posts::Column::CreatedAt, direction.order())
    .all(&db)
    .await?;
```

Because `SortDirection` derives `Deserialize`/`Serialize` with `"asc"`/`"desc"` renames, it deserializes directly from a query-string parameter (e.g. `?sort=desc` in a controller's `Query<T>` extractor).

## Result

You have a `Condition` built from readable, chainable method calls instead of raw Sea-ORM `Condition::all().add(...)` boilerplate, and it composes with `.filter()`, `.order_by()`, and — as the next guide shows — `query::paginate`.

## Next

- [Paginate results](/docs/how-to/paginate) using the condition you just built.
- [Query DSL & pagination reference](/docs/reference/query-pagination) for every operator's exact SQL output.
