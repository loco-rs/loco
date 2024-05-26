+++
title = "Cache"
description = ""
date = 2024-02-07T08:00:00+00:00
updated = 2024-02-07T08:00:00+00:00
draft = false
weight = 20
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

In your `app.rs` file, define a function named `override_context` function as a Hook in the `app.rs` file and import the `cache` module from `loco_rs`. 

Here's an example using an in-memory cache driver:

```rust
use loco_rs::cache;

async fn override_context(mut ctx: AppContext) -> Result<AppContext> {
    ctx.cache = cache::Cache::new(cache::drivers::inmem::new()).into();
    Ok(ctx)
}
```