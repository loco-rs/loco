---
title: Add a database to an existing app
description: "Turn an app generated with --db none into a database app: enable with-db, create the migration crate, and wire Hooks, the binaries, and config."
sidebar:
  order: 6
---

**Goal:** add a database to an app you generated **without** one.

Choosing no database at `loco new` is not a small switch — it turns off the `with-db` feature, which removes `AppContext::db`, the `migration` crate, the `models` module, and two required `Hooks` methods. No generator reverses it, so this page is the procedure. Budget half an hour.

:::note
If you are *starting* a new app, pick a database at the prompt instead. It is much cheaper than this page.
:::

## 1. Turn the feature back on

A `--db none` app pins `loco-rs` with default features off:

```toml
# Cargo.toml — before
loco-rs = { workspace = true, features = ["cli"] }
```

`with-db` is a default feature, so the fix is to stop disabling defaults. In `[workspace.dependencies]`, drop `default-features = false`, then add the database dependencies a db app ships with:

```toml
# Cargo.toml — after
[workspace.dependencies]
loco-rs = { version = "1.1" }        # no `default-features = false`

[dependencies]
loco-rs = { workspace = true }
migration = { path = "migration" }
sea-orm = { version = "2.0", features = [
  "sqlx-sqlite",
  "sqlx-postgres",
  "runtime-tokio-rustls",
  "macros",
] }
chrono = { version = "0.4" }
validator = { version = "0.20" }
uuid = { version = "1.6", features = ["v4"] }
```

Add `ts-rs = { version = "12", features = ["chrono-impl", "serde-compat"] }` as well if you want the typed DTO bindings — see [Build a typed React SPA](/docs/how-to/build-a-spa).

## 2. Create the `migration` crate

It is a sibling crate at `migration/`, referenced by path. Two files:

```toml
# migration/Cargo.toml
[package]
name = "migration"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
name = "migration"
path = "src/lib.rs"

[dependencies]
loco-rs = { workspace = true }

[dependencies.sea-orm-migration]
version = "2.0"
features = ["runtime-tokio-rustls"]
```

```rust
// migration/src/lib.rs
#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // inject-above (do not remove this comment)
        ]
    }
}
```

:::caution
Keep the `// inject-above (do not remove this comment)` line exactly as written. `cargo loco generate model` registers each new migration by injecting above it. Without the anchor the generator fails — loudly, which is the point: before Loco 1.1 it reported success and silently left the migration unregistered.
:::

## 3. Add the `models` module

```
src/models/
├── mod.rs              # pub mod _entities;
└── _entities/
    ├── mod.rs          # pub mod prelude;
    └── prelude.rs      # (empty for now)
```

Then declare it in `src/lib.rs`:

```rust
pub mod models;
pub mod dtos;   // only if you added ts-rs in step 1
```

`_entities/` is generated code — `cargo loco db entities` rewrites it from the live schema. Your own model logic goes in `src/models/<name>.rs` next to it, never inside `_entities/`.

## 4. Wire `Hooks` and the binaries

This is where the compiler errors the archetype hits come from, and they are all mechanical.

In `src/app.rs`, `boot` gains the `Migrator` type parameter:

```rust
use migration::Migrator;

async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult> {
    create_app::<Self, Migrator>(mode, environment, config).await
}
```

`with-db` also makes **two more `Hooks` methods required** — they have no default bodies, so `impl Hooks for App` will not compile until both exist:

```rust
use std::path::Path;

async fn truncate(_ctx: &AppContext) -> Result<()> {
    Ok(())
}

async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
    Ok(())
}
```

Empty bodies are fine to start; fill them in when you need them (see [Seed data](/docs/how-to/seed-data)).

Both binaries take the same parameter:

```rust
// src/bin/main.rs and src/bin/tool.rs
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
```

And `tests/mod.rs` gains its models module once you have model tests:

```rust
mod models;
```

## 5. Configure the connection

Add a `database:` block to **every** environment config — `config/development.yaml`, `config/test.yaml`, and `config/production.yaml`. Production takes no defaults, so it must read from the environment:

```yaml
# config/development.yaml
database:
  uri: <%= get_env(name="DATABASE_URL", default="sqlite://myapp_development.sqlite?mode=rwc") %>
  enable_logging: false
  connect_timeout: 500
  idle_timeout: 500
  min_connections: 1
  max_connections: 1
  auto_migrate: true
  dangerously_truncate: false
  dangerously_recreate: false
```

```yaml
# config/production.yaml
database:
  uri: <%= get_env(name="DATABASE_URL") %>
  auto_migrate: false
  dangerously_truncate: false
  dangerously_recreate: false
```

Use `sqlite://…?mode=rwc` for SQLite or `postgres://user:pass@host:5432/dbname` for Postgres. Never set `dangerously_truncate` or `dangerously_recreate` outside development and test.

## 6. Verify

```sh
cargo build                       # the Hooks/Migrator wiring compiles
cargo loco db status              # the connection works
cargo loco generate model post title:string! content:text
cargo loco db migrate
cargo loco db entities            # needs: cargo install sea-orm-cli
cargo loco start
```

If `generate model` reports that it cannot inject into `migration/src/lib.rs`, the anchor comment from step 2 is missing or was reformatted.

## Next

- [Add a model](/docs/how-to/add-model) — the field syntax and the normal workflow from here on
- [Query data](/docs/how-to/query-data)
- [Multi-database](/docs/how-to/multi-database) — if you need more than one connection
