+++
title = "Connect a second database"
description = "Attach an extra database connection, or several, using the built-in multi_db initializer."
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

Loco ships a ready-made [initializer](@/docs/how-to/add-middleware.md) for this: `MultiDbInitializer`, a named map of connections. It lives under `loco_rs::initializers::multi_db` and is gated behind the `with-db` feature. Each config entry accepts the same keys as the primary `database:` block — see [Configuration § database](@/docs/reference/configuration.md) for the full list (`uri`, `enable_logging`, `min_connections`, `max_connections`, `connect_timeout`, `idle_timeout`, `acquire_timeout`, `auto_migrate`, `dangerously_truncate`, `dangerously_recreate`, `run_on_start`).

## Configure each named database

Add one or more entries under a `multi_db` map, nested under the top-level `initializers` key in your environment config. A single extra connection is just a one-entry map:

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
```

Add more entries to open more connections:

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

## Register the initializer

```rust
use loco_rs::app::{AppContext, Initializer};

async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
    let initializers: Vec<Box<dyn Initializer>> = vec![
        Box::new(loco_rs::initializers::multi_db::MultiDbInitializer),
    ];

    Ok(initializers)
}
```

## Look connections up by name

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

`ctx.db` remains your app's primary connection (used for auto-migration, boot-time checks, etc.); the extra connection(s) arrive purely through the `MultiDb` axum `Extension` and only in handlers that ask for them.

## Migrating from `extra_db`

`ExtraDbInitializer` has been removed in favor of `MultiDbInitializer`. To migrate:

- Config: former `initializers.extra_db: { ... }` becomes `initializers.multi_db: { <name>: { ... } }` — pick a name for your connection and nest the same keys under it.
- Registration: swap `Box::new(loco_rs::initializers::extra_db::ExtraDbInitializer)` for `Box::new(loco_rs::initializers::multi_db::MultiDbInitializer)`.
- Handlers: change `Extension(db): Extension<DatabaseConnection>` to `Extension(multi_db): Extension<MultiDb>`, then look up the connection with `let db = multi_db.get("<name>")?;`.

## Next

- [Configuration reference](@/docs/reference/configuration.md) for every key a `database:`-shaped block accepts.
- [Add middleware](@/docs/how-to/add-middleware.md) for how the `initializers`/middleware hooks work.
