+++
title = "Connect a second database"
description = "Attach an extra database connection, or several, using the built-in extra_db and multi_db initializers."
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

**Goal:** query a second (or third+) database from a controller, alongside the app's primary `ctx.db` connection — for example, a read replica, a legacy database, or per-tenant databases.

Loco ships two ready-made [initializers](@/docs/extras/pluggability.md#initializers) for this: `ExtraDbInitializer` (exactly one extra connection) and `MultiDbInitializer` (a named map of connections). Both live under `loco_rs::initializers::{extra_db, multi_db}` and are gated behind the `with-db` feature. Each config entry accepts the same keys as the primary `database:` block — see [Configuration § database](@/docs/reference/configuration.md) for the full list (`uri`, `enable_logging`, `min_connections`, `max_connections`, `connect_timeout`, `idle_timeout`, `acquire_timeout`, `auto_migrate`, `dangerously_truncate`, `dangerously_recreate`, `run_on_start`).

## Option A: one extra database

### 1. Configure it

Add an `extra_db` entry under the top-level `initializers` key in your environment config:

```yaml
initializers:
  extra_db:
    uri: postgres://loco:loco@localhost:5432/legacy_app
    enable_logging: false
    connect_timeout: 500
    idle_timeout: 500
    min_connections: 1
    max_connections: 1
    auto_migrate: false
    dangerously_truncate: false
    dangerously_recreate: false
```

### 2. Register the initializer

```rust
use loco_rs::app::{AppContext, Initializer};

async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
    let initializers: Vec<Box<dyn Initializer>> = vec![
        Box::new(loco_rs::initializers::extra_db::ExtraDbInitializer),
    ];

    Ok(initializers)
}
```

`ExtraDbInitializer` reads the `extra_db` config, opens the connection with `db::connect`, and layers it onto the router as an axum `Extension<DatabaseConnection>`.

### 3. Use it in a controller

```rust
use sea_orm::{DatabaseConnection, EntityTrait};
use axum::{response::IntoResponse, Extension};

pub async fn list(
    State(ctx): State<AppContext>,
    Extension(legacy_db): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse> {
    let res = Entity::find().all(&legacy_db).await;
    format::json(res)
}
```

## Option B: several named databases

If you need more than one secondary connection, use `multi_db` instead — it's a map of arbitrary names to database configs.

### 1. Configure each named database

```yaml
initializers:
  multi_db:
    secondary_db:
      uri: postgres://loco:loco@localhost:5432/loco_app
      enable_logging: false
      connect_timeout: 500
      idle_timeout: 500
      min_connections: 1
      max_connections: 1
      auto_migrate: false
      dangerously_truncate: false
      dangerously_recreate: false
    third_db:
      uri: postgres://loco:loco@localhost:5432/loco_app_reporting
      enable_logging: false
      connect_timeout: 500
      idle_timeout: 500
      min_connections: 1
      max_connections: 1
      auto_migrate: false
      dangerously_truncate: false
      dangerously_recreate: false
```

### 2. Register the initializer

```rust
use loco_rs::app::{AppContext, Initializer};

async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
    let initializers: Vec<Box<dyn Initializer>> = vec![
        Box::new(loco_rs::initializers::multi_db::MultiDbInitializer),
    ];

    Ok(initializers)
}
```

### 3. Look connections up by name

`MultiDbInitializer` layers a `loco_rs::db::MultiDb` (a thin `HashMap<String, DatabaseConnection>` wrapper) as an axum `Extension`:

```rust
use sea_orm::EntityTrait;
use axum::{response::IntoResponse, Extension};
use loco_rs::db::MultiDb;

pub async fn list(
    State(ctx): State<AppContext>,
    Extension(multi_db): Extension<MultiDb>,
) -> Result<impl IntoResponse> {
    let third_db = multi_db.get("third_db")?;
    let res = Entity::find().all(third_db).await;
    format::json(res)
}
```

`multi_db.get(name)` returns an error if that key isn't configured — no silent `None`/panic.

## Result

`ctx.db` remains your app's primary connection (used for auto-migration, boot-time checks, etc.); the extra connection(s) arrive purely through axum `Extension` and only in handlers that ask for them. Register **either** `ExtraDbInitializer` **or** `MultiDbInitializer` — they read different config keys (`extra_db` vs `multi_db`) and layer different extension types, so pick the one matching the number of secondary databases you need.

## Next

- [Configuration reference](@/docs/reference/configuration.md) for every key a `database:`-shaped block accepts.
- [Pluggability § Initializers](@/docs/extras/pluggability.md#initializers) for how the `initializers` hook itself works.
