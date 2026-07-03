+++
title = "Use the cache"
description = "Configure a cache driver (null/in-memory/Redis) and use get/insert/get_or_insert with expiry, ping, and clear."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 31
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

Goal: cache a value (a computed result, an external API response, a hot query) behind a key, with an optional TTL, using a driver you can swap per environment.

`ctx.cache` is available in every controller, task, and worker. Values are serialized as JSON strings under the hood, so anything `Serialize + DeserializeOwned` can go in.

## 1. Configure a driver

Cache drivers are configured entirely in YAML — no code changes needed to switch drivers between environments.

```yaml
# config/development.yaml — fast, disposable, no external dependency
cache:
  kind: InMem
  max_capacity: 33554432 # optional, bytes; default 32MiB (32 * 1024 * 1024)
```

```yaml
# config/production.yaml — shared across processes
cache:
  kind: Redis
  uri: "{{ get_env(name='REDIS_CACHE_URL', default='redis://127.0.0.1:6379') }}"
  max_size: 10 # required — max pool connections
```

```yaml
# omit the `cache` key entirely, or set explicitly — this is the default
cache:
  kind: Null
```

`InMem` needs the `cache_inmem` feature (on by default); `Redis` needs `cache_redis` (off by default — add it to your `Cargo.toml`). See the [feature flags reference](@/docs/reference/feature-flags.md).

If you omit `cache` from the config file altogether, Loco silently falls back to the **`Null` driver**: `get()` always returns `None`, and every mutating operation (`insert`, `insert_with_expiry`, `remove`, `clear`, `ping`) returns an error. This is a fail-fast default for "you haven't configured a real cache" — don't ship it to production by accident.

## 2. Insert and read values

```rust
use loco_rs::cache;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

async fn cache_basics(ctx: &AppContext) -> Result<()> {
    ctx.cache.insert("greeting", &"hello".to_string()).await?;

    let user = User { name: "Alice".to_string(), age: 30 };
    ctx.cache.insert("user:1", &user).await?;

    let greeting: Option<String> = ctx.cache.get("greeting").await?;
    let cached_user: Option<User> = ctx.cache.get("user:1").await?;

    let exists: bool = ctx.cache.contains_key("user:1").await?;

    ctx.cache.remove("greeting").await?;
    Ok(())
}
```

## 3. Set a TTL with `insert_with_expiry`

```rust
use std::time::Duration;

ctx.cache
    .insert_with_expiry("session:abc", &token, Duration::from_secs(300))
    .await?;
```

## 4. Cache the result of a computation with `get_or_insert`

`get_or_insert` returns the cached value if present, otherwise runs the given future, stores the result, and returns it. `get_or_insert_with_expiry` does the same but attaches a TTL to the freshly-computed value.

```rust
let expensive_report = ctx
    .cache
    .get_or_insert::<Report, _>("report:daily", async {
        build_daily_report(ctx).await
    })
    .await?;
```

```rust
use std::time::Duration;

let expensive_report = ctx
    .cache
    .get_or_insert_with_expiry::<Report, _>(
        "report:daily",
        Duration::from_secs(3600),
        async { build_daily_report(ctx).await },
    )
    .await?;
```

## 5. Health-check and clear

```rust
// Fails if the backing store (e.g. Redis) is unreachable.
ctx.cache.ping().await?;

// Wipe the cache.
ctx.cache.clear().await?;
```

> **Redis caveat:** `Cache::clear()` on the Redis driver issues **`FLUSHDB`** — it flushes the *entire* Redis logical database, not just the keys your app put there. If other data (session store, queue, another app) shares that same Redis DB/instance, `clear()` will delete it too. Point cache at its own Redis DB (`redis://host:6379/1`, a separate `db` index) if you need to isolate it, and treat `clear()` as a blunt, whole-database operation.

## 6. Verify

```rust
#[tokio::test]
async fn can_get_or_insert() {
    let app_ctx = get_app_context().await; // your test AppContext
    let key = "loco";

    assert_eq!(app_ctx.cache.get::<String>(key).await.unwrap(), None);

    let result = app_ctx
        .cache
        .get_or_insert::<String, _>(key, async { Ok("loco-cache-value".to_string()) })
        .await
        .unwrap();

    assert_eq!(result, "loco-cache-value");
    assert_eq!(
        app_ctx.cache.get::<String>(key).await.unwrap(),
        Some("loco-cache-value".to_string())
    );
}
```

## Reference

- Every `cache:` YAML key (`kind`, `max_capacity`, `uri`, `max_size`): [Configuration reference § cache](@/docs/reference/configuration.md#cache)
- `cache_inmem` / `cache_redis` feature flags: [Feature flags reference](@/docs/reference/feature-flags.md)
