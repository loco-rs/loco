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

* Create a clean branch in your code repo.
* Update the Loco version in your main `Cargo.toml`
* Consult with the [CHANGELOG](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md) to find breaking changes and refactorings you should do (if any).
* Run `cargo loco doctor` inside your project to verify that your app and environment is compatible with the new version

As always, if anything turns wrong, [open an issue](https://github.com/loco-rs/loco/issues) and ask for help.

## Major Loco dependencies

Loco is built on top of great libraries. It's wise to be mindful of their versions in new releases of Loco, and their individual changelogs.

These are the major ones:

* [SeaORM](https://www.sea-ql.org/SeaORM), [CHANGELOG](https://github.com/SeaQL/sea-orm/blob/master/CHANGELOG.md)
* [Axum](https://github.com/tokio-rs/axum), [CHANGELOG](https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md)


## Upgrade from 0.13.x to 0.14.x

### Upgrading from Axum 0.7 to 0.8

PR: [#1130](https://github.com/loco-rs/loco/pull/1130)
The upgrade to Axum 0.8 introduces a breaking change. For more details, refer to the [announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0).
#### Steps to Upgrade
* In your `Cargo.toml`, update the Axum version from `0.7.5` to `0.8.1`.
* Replace use `axum::async_trait`; with use `async_trait::async_trait;`. For more information, see [here](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0#async_trait-removal).
* The URL parameter syntax has changed. Refer to [this section](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0#path-parameter-syntax-changes) for the updated syntax. The new path parameter format is:
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

## Upgrade from 0.14.x to 0.15.x