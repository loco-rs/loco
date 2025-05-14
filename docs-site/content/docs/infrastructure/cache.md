+++
title = "Cache"
description = ""
date = 2024-02-07T08:00:00+00:00
updated = 2025-04-22T08:00:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

`Loco` provides a cache layer to improve application performance by storing frequently accessed data.

## Supported Cache Drivers

Loco supports several cache drivers out of the box:

1. **Null Cache**: A no-op cache that doesn't actually store anything (default)
2. **In-Memory Cache**: A local in-memory cache using the `moka` crate
3. **Redis Cache**: A distributed cache using Redis

## Default Behavior

By default, `Loco` initializes a `Null` cache driver. The Null driver implements the cache interface but doesn't actually store any data:

- `get()` operations always return `None`
- Other operations like `insert()`, `remove()`, etc. return errors with a message indicating the operation is not supported

If you use the cache functionality without configuring a proper cache driver, many operations will result in errors. It's recommended to configure a real cache driver for production use.

## Configuring Cache Drivers

You can configure your preferred cache driver in your application's configuration files (e.g., `config/development.yaml`).

### Configuration Examples

#### Null Cache (Default)

```yaml
cache:
  kind: Null
```

#### In-Memory Cache
feature `cache_inmem` enable by default
```yaml
cache:
  kind: InMem
  max_capacity: 33554432 # 32MiB (default if not specified)
```

#### Redis Cache
feature `cache_redis` should be enabled
```yaml
cache:
  kind: Redis
  uri: "redis://localhost:6379"
  max_size: 10 # Maximum number of connections in the pool
```

If no cache configuration is provided, the `Null` cache will be used by default.

## Using the Cache

All items are cached as serialized values with string keys.

```rust
use std::time::Duration;
use loco_rs::cache;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

async fn test_cache(ctx: AppContext) -> Result<()> {
    // Insert a simple string value
    ctx.cache.insert("string_key", "simple value").await?;

    // Insert a structured value
    let user = User { name: "Alice".to_string(), age: 30 };
    ctx.cache.insert("user:1", &user).await?;

    // Insert with expiration
    ctx.cache.insert_with_expiry("expiring_key", "temporary value", Duration::from_secs(300)).await?;

    // Retrieve a string value
    let string_value = ctx.cache.get::<String>("string_key").await?;

    // Retrieve a structured value
    let user = ctx.cache.get::<User>("user:1").await?;

    // Remove a value
    ctx.cache.remove("string_key").await?;

    // Check if a key exists
    let exists = ctx.cache.contains_key("user:1").await?;

    // Get or insert (retrieve if exists, otherwise compute and store)
    let lazy_value = ctx.cache.get_or_insert::<String, _>("lazy_key", async {
        Ok("computed value".to_string())
    }).await?;

    Ok(())
}
```

See the [Cache API](https://docs.rs/loco-rs/latest/loco_rs/cache/struct.Cache.html) docs for more examples.
