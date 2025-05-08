+++
title = "Upgrades"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

## What to do when a new Loco version is out?

- Create a clean branch in your code repo.
- Update the Loco version in your main `Cargo.toml`
- Consult with the [CHANGELOG](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md) to find breaking changes and refactorings you should do (if any).
- Run `cargo loco doctor` inside your project to verify that your app and environment is compatible with the new version

As always, if anything turns wrong, [open an issue](https://github.com/loco-rs/loco/issues) and ask for help.

## Major Loco dependencies

Loco is built on top of great libraries. It's wise to be mindful of their versions in new releases of Loco, and their individual changelogs.

These are the major ones:

- [SeaORM](https://www.sea-ql.org/SeaORM), [CHANGELOG](https://github.com/SeaQL/sea-orm/blob/master/CHANGELOG.md)
- [Axum](https://github.com/tokio-rs/axum), [CHANGELOG](https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md)

## Upgrade from 0.15.x to 0.16.x

### Use `AppContext` instead of `Config` in `init_logger` in the `Hooks` trait

PR: [#1418](https://github.com/loco-rs/loco/pull/1418)

If you are supplying an implementation of `init_logger` in your `impl` of the `Hooks` trait in order to set up your own logging, you will need to make the following change:

```diff
- fn init_logger(config: &config::Config, env: &Environment) -> Result<bool> {
+ fn init_logger(ctx: &AppContext) -> Result<bool> {
```

Any code in your `init_logger` implementation that makes use of the `config` can access it through `ctx.config`. In addition, you will also be able to access anything else in the `AppContext`, such as the new `shared_store`. The `env` parameter is also removed, as that is accessible from the `AppContext` as `ctx.environment`.

### Swap to validators builtin email validation

PR: [#1359](https://github.com/loco-rs/loco/pull/1359)

Swap from using the loco custom email validator, to the builtin email validator from `validator`.

```diff
- #[validate(custom (function = "validation::is_valid_email"))]
+ #[validate(email(message = "invalid email"))]
  pub email: String,
```

### Job system

PR: [#1384](https://github.com/loco-rs/loco/pull/1384)
PR: [#1396](https://github.com/loco-rs/loco/pull/1396)

Two major changes have been made to the background job system:

1. The Redis provider is no longer Sidekiq-compatible and uses a custom implementation
2. All providers (Redis, PostgreSQL, SQLite) now support tag-based job filtering

#### What Changed

##### Removing Sidekiq Compatibility

The Redis background job system has been completely refactored, replacing the Sidekiq-compatible implementation with a new custom implementation. This provides greater flexibility and improved performance, but means:

- Jobs pushed from older Loco versions (pre-0.16) will not be recognized or processed
- The Redis data structures have changed entirely
- There is no automatic migration path for existing queued jobs

##### Adding Job Filtering

A new tag-based job filtering system has been added to all background worker providers:

- Workers can now specify which tags they're interested in processing
- Jobs can be tagged when enqueued
- Workers with no tags only process untagged jobs, while tagged workers process jobs with matching tags
- The same API is used across all providers

#### How to Upgrade

To upgrade to the new job system:

1. **Process existing jobs**:

   - Make sure all jobs in your queue are processed/completed before upgrading

2. **Clean up old data**:

   - For Redis: Flush the Redis database used for jobs (`FLUSHDB` command)
   - For PostgreSQL: Drop the job queue tables
   - For SQLite: Delete the job queue tables

3. **Update Loco**:
   - Update to Loco 0.16+
   - Loco will automatically create new job tables with the correct schema on first run

### Generic Cache

PR: [#1385](https://github.com/loco-rs/loco/pull/1385)

The cache API has been refactored to support storing and retrieving any serializable type, not just strings. This is a breaking change that requires updates to your code:

#### Breaking Changes:

1. **Type Parameters Required**: All cache methods now require explicit type parameters
2. **Method Signatures**: Some method signatures have changed to support generics
3. **Object Serialization**: Any type you store must implement `Serialize` and `Deserialize` from serde

#### Migration Guide:

**Before:**

```rust
// Get a string value from cache
let value = cache.get("key").await?;

// Insert or get with callback
let value = app_ctx.cache.get_or_insert("key", async {
    Ok("value".to_string())
}).await.unwrap();

// Insert or get with expiry
let value = app_ctx.cache.get_or_insert_with_expiry("key", Duration::from_secs(300), async {
    Ok("value".to_string())
}).await.unwrap();
```

**After:**

```rust
// Get a string value from cache - specify the type
let value = cache.get::<String>("key").await?;

// Direct insert with any serializable type
cache.insert("key", &"value".to_string()).await?;

// Insert or get with callback - specify return type
let value = app_ctx.cache.get_or_insert::<String, _>("key", async {
    Ok("value".to_string())
}).await.unwrap();

// Store complex types
#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

let user = app_ctx.cache.get_or_insert_with_expiry::<User, _>(
    "user:1",
    Duration::from_secs(300),
    async {
        Ok(User { name: "Alice".to_string(), age: 30 })
    }
).await.unwrap();
```

#### Implementing for Custom Types:

For your custom types to work with the cache, ensure they implement `Serialize` and `Deserialize`:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct MyType {
    // fields...
}
```

### Authentication Error Handling

Authentication error handling has been improved to better distinguish between actual authorization failures and system errors:

1. **System errors now return 500**: Database errors during authentication now return Internal Server Error (500) instead of Unauthorized (401)
2. **Improved error logging**: Authentication errors are now logged with detailed messages using `tracing::error`
3. **Message changes**: Generic error messages have been updated from "other error: '{e}'" to "could not authorize"

#### Migration Guide:

If you have code that relies on database errors during authentication returning 401 status codes, you'll need to update your error handling. Any code expecting a 401 for database connectivity issues should now handle 500 responses as well.

Client applications should be prepared to handle both 401 and 500 status codes during authentication failures, with 401 indicating authorization problems and 500 indicating system errors.

## Upgrade from 0.14.x to 0.15.x

### Upgrade validator crate

PR: [#1199](https://github.com/loco-rs/loco/pull/1199)

Update the `validator` crate version in your `Cargo.toml`:

From

```
validator = { version = "0.19" }
```

To

```
validator = { version = "0.20" }
```

### User claims

PR: [#1159](https://github.com/loco-rs/loco/pull/1159)

- Flattened (De)Serialization of Custom User Claims:
  The `claims` field in `UserClaims` has changed from `Option<Value>` to `Map<String, Value>`.

- Mandatory Map Value in `generate_token` function:
  When calling `generate_token`, the `Map<String, Value>` argument is now required. If you are not using custom claims, pass an empty map (`serde_json::Map::new()`).

- Updated generate_token Signature:
  The `generate_token` function now takes `expiration` as a value instead of a reference.

### Pagination Response

PR: [#1197](https://github.com/loco-rs/loco/pull/1197)

The pagination response now includes the `total_items` field, providing the total number of items available.

```JSON
{"results":[],"pagination":{"page":0,"page_size":0,"total_pages":0,"total_items":0}}
```

### Explicit id in migrations

PR: [#1268](https://github.com/loco-rs/loco/pull/1268)

Migrations using `create_table` now require `("id", ColType::PkAuto)`, new migrations will have this field automatically added.

```diff
  async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(m, "movies",
            &[
+           ("id", ColType::PkAuto),
            ("title", ColType::StringNull),
            ],
            &[
            ("user", ""),
            ]
        ).await
    }
```

## Upgrade from 0.13.x to 0.14.x

### Upgrading from Axum 0.7 to 0.8

PR: [#1130](https://github.com/loco-rs/loco/pull/1130)
The upgrade to Axum 0.8 introduces a breaking change. For more details, refer to the [announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0).

#### Steps to Upgrade

- In your `Cargo.toml`, update the Axum version from `0.7.5` to `0.8.1`.
- Replace use `axum::async_trait`; with use `async_trait::async_trait;`. For more information, see [here](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0#async_trait-removal).
- The URL parameter syntax has changed. Refer to [this section](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0#path-parameter-syntax-changes) for the updated syntax. The new path parameter format is:
  The path parameter syntax has changed from `/:single` and `/*many` to `/{single}` and `/{*many}`.

### Extending the `boot` Function Hook

PR: [#1143](https://github.com/loco-rs/loco/pull/1143)

The `boot` hook function now accepts an additional Config parameter. The function signature has changed from:

From

```rust
async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult> {
     create_app::<Self, Migrator>(mode, environment).await
}
```

To:

```rust
async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult> {
     create_app::<Self, Migrator>(mode, environment, config).await
}
```

Make sure to import the `Config` type as needed.

### Upgrade validator crate

PR: [#993](https://github.com/loco-rs/loco/pull/993)

Update the `validator` crate version in your `Cargo.toml`:

From

```
validator = { version = "0.18" }
```

To

```
validator = { version = "0.19" }
```

### Extend truncate and seed hooks

PR: [#1158](https://github.com/loco-rs/loco/pull/1158)

The `truncate` and `seed` functions now receive `AppContext` instead of `DatabaseConnection` as their argument.

From

```rust
async fn truncate(db: &DatabaseConnection) -> Result<()> {}
async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {}
```

To

```rust
async fn truncate(ctx: &AppContext) -> Result<()> {}
async fn seed(_ctx: &AppContext, base: &Path) -> Result<()> {}
```

Impact on Testing:

Testing code involving the seed function must also be updated accordingly.

from:

```rust
async fn load_page() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx.db).await.unwrap();
        ...
    })
    .await;
}
```

to

```rust
async fn load_page() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        ...
    })
    .await;
}
```
