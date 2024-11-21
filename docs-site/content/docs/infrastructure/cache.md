+++
title = "Cache"
description = ""
date = 2024-02-07T08:00:00+00:00
updated = 2024-02-07T08:00:00+00:00
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

`Loco` provides an cache layer to improve application performance by storing frequently accessed data.

## Default Behavior

By default, `Loco` initializes a `Null` cache driver. This means any interaction with the cache will return an error, effectively bypassing the cache functionality. 

## Enabling Caching

To enable caching and configure a specific cache driver, you can replace the default `Null` driver with your preferred implementation.

In your `app.rs` file, define a function named `after_context` function as a Hook in the `app.rs` file and import the `cache` module from `loco_rs`. 

Here's an example using an in-memory cache driver:

```rust
use loco_rs::cache;

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    Ok(AppContext {
        cache: cache::Cache::new(cache::drivers::inmem::new()).into(),
        ..ctx
    })
}
```

## Caching Items

All items are cached as &str values and keys.

```rust
use loco_rs::cache;

async fn test_cache(ctx: AppContext) {
    
    // insert an item into the cache
    ctx.cache.insert("key", "value").await;
    
    // insert an item into the cache that expires after x seconds
    ctx.cache.insert_with_expiry("key", "value", 300).await;
    
    // retrieve an item from cache
    let value = ctx.cache.get("key").await.unwrap();
    
}
```

See the [Cache API](https://docs.rs/loco-rs/latest/loco_rs/cache/struct.Cache.html) docs for more examples.