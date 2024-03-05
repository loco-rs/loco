+++
title = "Multiple Db"
description = ""
date = 2024-03-01T18:10:00+00:00
updated = 2024-03-01T18:10:00+00:00
draft = false
weight = 31
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

`Loco` enables you to work with more than one database and share instances across your application.

To set up an additional database, begin with database connections and configuration. The recommended approach is to navigate to your configuration file and add the following under [settings](@/docs/getting-started/config.md#settings):

```yaml
settings:
  extra_db:
    uri: postgres://loco:loco@localhost:5432/loco_app
    enable_logging: false
    connect_timeout: 500
    idle_timeout: 500
    min_connections: 1
    max_connections: 1
    auto_migrate: true
    dangerously_truncate: false
    dangerously_recreate: false
```


After configuring the database, import [loco-extras](https://crates.io/crates/loco-extras) and enable the `initializer-extra-db` feature in your Cargo.toml:
```toml
loco-extras = { version = "*", features = ["initializer-extra-db"] }
```

Next load this [initializer](@/docs/the-app/initializers.md) into `initializers` hook like this example

```rs
async fn initializers(ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        let  initializers: Vec<Box<dyn Initializer>> = vec![
            Box::new(loco_extras::initializers::extra_db::ExtraDbInitializer),
        ];

        Ok(initializers)
    }
```

Now, you can use the secondary database in your controller:

```rust
use sea_orm::DatabaseConnection;
use axum::{response::IntoResponse, Extension};

pub async fn list(
    State(ctx): State<AppContext>,
    Extension(secondary_db): Extension<DatabaseConnection>,
) -> Result<impl IntoResponse> {
  let res = Entity::find().all(&secondary_db).await;
}
```

# Many Database Connections

To connect more than two different databases, load the feature `initializer-multi-db` in [loco-extras](https://crates.io/crates/loco-extras):
```toml
loco-extras = { version = "*", features = ["initializer-multi-db"] }
```

The database configuration should look like this:
```yaml
settings:
  multi_db: 
    secondary_db:      
      uri: postgres://loco:loco@localhost:5432/loco_app
      enable_logging: false      
      connect_timeout: 500      
      idle_timeout: 500      
      min_connections: 1      
      max_connections: 1      
      auto_migrate: true      
      dangerously_truncate: false      
      dangerously_recreate: false
    third_db:      
      uri: postgres://loco:loco@localhost:5432/loco_app
      enable_logging: false      
      connect_timeout: 500      
      idle_timeout: 500      
      min_connections: 1      
      max_connections: 1      
      auto_migrate: true      
      dangerously_truncate: false      
      dangerously_recreate: false
```

Next load this [initializer](@/docs/the-app/initializers.md) into `initializers` hook like this example

```rs
async fn initializers(ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        let  initializers: Vec<Box<dyn Initializer>> = vec![
            Box::new(loco_extras::initializers::multi_db::MultiDbInitializer),
        ];

        Ok(initializers)
    }
```

Now, you can use the multiple databases in your controller:

```rust
use sea_orm::DatabaseConnection;
use axum::{response::IntoResponse, Extension};
use loco_rs::db::MultiDb;

pub async fn list(
    State(ctx): State<AppContext>,
    Extension(multi_db): Extension<MultiDb>,
) -> Result<impl IntoResponse> {
  let third_db = multi_db.get("third_db")?;
  let res = Entity::find().all(third_db).await;
}
```