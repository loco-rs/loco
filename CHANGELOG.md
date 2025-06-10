# Changelog

## Unreleased

* chore: improve readability and performance by using map_err in Model. [https://github.com/loco-rs/loco/pull/1311](https://github.com/loco-rs/loco/pull/1311)
* Allow testing the controller by passing a cookie. [https://github.com/loco-rs/loco/pull/1326](https://github.com/loco-rs/loco/pull/1326)
* Support BigInt in the scaffold Array. [https://github.com/loco-rs/loco/pull/1304](https://github.com/loco-rs/loco/pull/1304)
* Add `escape` Tera function to the scaffold list template. [https://github.com/loco-rs/loco/pull/1337](https://github.com/loco-rs/loco/pull/1337)
* Return a specific error when logging in with a non-existent email. [https://github.com/loco-rs/loco/pull/1336](https://github.com/loco-rs/loco/pull/1336)
* Return a specific error when trying to verify with an invalid token. [https://github.com/loco-rs/loco/pull/1340](https://github.com/loco-rs/loco/pull/1340)
* Clippy 1.86. [https://github.com/loco-rs/loco/pull/1353](https://github.com/loco-rs/loco/pull/1353)
* Fix the DB creation. [https://github.com/loco-rs/loco/pull/1352](https://github.com/loco-rs/loco/pull/1352)
* YAML responses. [https://github.com/loco-rs/loco/pull/1360](https://github.com/loco-rs/loco/pull/1360)
* Swap to the validators' built-in email validation. [https://github.com/loco-rs/loco/pull/1359](https://github.com/loco-rs/loco/pull/1359)
* Cancellation tokens for the Postgres and SQLite background workers. [https://github.com/loco-rs/loco/pull/1365](https://github.com/loco-rs/loco/pull/1365)
* docs: testing auth routes. [https://github.com/loco-rs/loco/pull/1303](https://github.com/loco-rs/loco/pull/1303)
* Add comprehensive tests for the task module. [https://github.com/loco-rs/loco/pull/1386](https://github.com/loco-rs/loco/pull/1386)
* Add comprehensive test coverage for the data module. [https://github.com/loco-rs/loco/pull/1387](https://github.com/loco-rs/loco/pull/1387)
* Add validator extractors test suite. [https://github.com/loco-rs/loco/pull/1388](https://github.com/loco-rs/loco/pull/1388)
* **Breaking changes** Replace sidekiq job management. [https://github.com/loco-rs/loco/pull/1384](https://github.com/loco-rs/loco/pull/1384)
* **Breaking changes** Add generic type support to the Cache API. [https://github.com/loco-rs/loco/pull/1385](https://github.com/loco-rs/loco/pull/1385)
* Adding cache redis driver + configuration instead of enabling from code. [https://github.com/loco-rs/loco/pull/1389](https://github.com/loco-rs/loco/pull/1389)
* Ability to configure pragma for SQLite. [https://github.com/loco-rs/loco/pull/1346](https://github.com/loco-rs/loco/pull/1346)
* **Breaking changes** swap to validators builtin email validation. [https://github.com/loco-rs/loco/pull/1359](https://github.com/loco-rs/loco/pull/1359)
* Optimize worker tag filtering string handling. [https://github.com/loco-rs/loco/pull/1396](https://github.com/loco-rs/loco/pull/1396)
* Add test coverage for db.rs. [https://github.com/loco-rs/loco/pull/1400](https://github.com/loco-rs/loco/pull/1400)
* Allow storage of arbitrary custom objects in AppContext. [https://github.com/loco-rs/loco/pull/1404](https://github.com/loco-rs/loco/pull/1404)
* Improve deployment generator CLI. [https://github.com/loco-rs/loco/pull/1413](https://github.com/loco-rs/loco/pull/1413)
* Move auth and validate to the extractor folder. [https://github.com/loco-rs/loco/pull/1414](https://github.com/loco-rs/loco/pull/1414)
* Hot reload on extended Tera templates. [https://github.com/loco-rs/loco/pull/1416](https://github.com/loco-rs/loco/pull/1416)
* **Breaking changes** Update the `init_logger` to use `AppContext` instead of config. [https://github.com/loco-rs/loco/pull/1418](https://github.com/loco-rs/loco/pull/1418)
* Support embedded assets. [https://github.com/loco-rs/loco/pull/1427](https://github.com/loco-rs/loco/pull/1427)
* **Removed dependencies:**
  - [`hyper`](https://github.com/loco-rs/loco/pull/1430)
  - [`thousands`](https://github.com/loco-rs/loco/pull/1431)
  - [`cfg-if`](https://github.com/loco-rs/loco/pull/1432)
  - [`reqwest`](https://github.com/loco-rs/loco/pull/1434)
- **Dependency updates:**
  - Bumped [`tokio`] to `1.45` and [`tokio-util`] to `0.7` ([#1435](https://github.com/loco-rs/loco/pull/1435))
  - Bumped [`colored`] to `3.0` ([#1437](https://github.com/loco-rs/loco/pull/1437))
  - Bumped [`rand`] to `0.9` ([#1439](https://github.com/loco-rs/loco/pull/1439))
  - Bumped [`duct`] to `1.0` ([#1438](https://github.com/loco-rs/loco/pull/1438))
  - Bumped [`redis`] to `0.31`, [`bb8`] to `0.9`, and [`bb8-redis`] to `0.23` ([commit `7e7be`](https://github.com/loco-rs/loco/commit/7e7bebe15f74c377c93d979aab41c52eb871d667))
  - Updated Loco template crates ([#1440](https://github.com/loco-rs/loco/pull/1440))
 * Support custom flags from `sea-orm entity`. [https://github.com/loco-rs/loco/pull/1442](https://github.com/loco-rs/loco/pull/1442)
 * Better `loco new` cleanup folders. [https://github.com/loco-rs/loco/pull/1429](https://github.com/loco-rs/loco/pull/1429)
 * Remove legacy mailer derive macro code. [https://github.com/loco-rs/loco/pull/1472](https://github.com/loco-rs/loco/pull/1472)



## v0.15.0

* Added total_items to pagination view & response. [https://github.com/loco-rs/loco/pull/1197](https://github.com/loco-rs/loco/pull/1197)
* Flatten (de)serialization of custom user claims. [https://github.com/loco-rs/loco/pull/1159](https://github.com/loco-rs/loco/pull/1159)
* Updated validator to 0.20. [https://github.com/loco-rs/loco/pull/1199](https://github.com/loco-rs/loco/pull/1199)
* Scaffold v2. [https://github.com/loco-rs/loco/pull/1209](https://github.com/loco-rs/loco/pull/1209)
* Fix generator Docker deployment to support both server-side and client-side rendering. [https://github.com/loco-rs/loco/pull/1227](https://github.com/loco-rs/loco/pull/1227)
* Docs: num_workers worker configuration. [https://github.com/loco-rs/loco/pull/1242](https://github.com/loco-rs/loco/pull/1242)
* Smoother model validations. [https://github.com/loco-rs/loco/pull/1233](https://github.com/loco-rs/loco/pull/1233)
* Docs: num_workers worker configuration. [https://github.com/loco-rs/loco/pull/1242](https://github.com/loco-rs/loco/pull/1242)
* Ignore SQLite WAL and SHM files and update Cargo watch crate docs. [https://github.com/loco-rs/loco/pull/1254](https://github.com/loco-rs/loco/pull/1254)
* Remove fs-err crate. [https://github.com/loco-rs/loco/pull/1253](https://github.com/loco-rs/loco/pull/1253)
* Allows to run scheduler as part of cargo loco start. [https://github.com/loco-rs/loco/pull/1247](https://github.com/loco-rs/loco/pull/1247)
* Added prefix and route nesting to AppRoutes. [https://github.com/loco-rs/loco/pull/1241](https://github.com/loco-rs/loco/pull/1241)
* Replace hyper crate with axum. [https://github.com/loco-rs/loco/pull/1258](https://github.com/loco-rs/loco/pull/1258)
* Remove mime crate. [https://github.com/loco-rs/loco/pull/1256](https://github.com/loco-rs/loco/pull/1256)
* Support async tests. [https://github.com/loco-rs/loco/pull/1237](https://github.com/loco-rs/loco/pull/1237)
* Change job queue status from cli.  [https://github.com/loco-rs/loco/pull/1228](https://github.com/loco-rs/loco/pull/1228)
* Handle panics in queue worker. [https://github.com/loco-rs/loco/pull/1274](https://github.com/loco-rs/loco/pull/1274)
* Schema with defaults. [https://github.com/loco-rs/loco/pull/1273](https://github.com/loco-rs/loco/pull/1273)
* Add data subsystem. [https://github.com/loco-rs/loco/pull/1267](https://github.com/loco-rs/loco/pull/1267)
* Add "endpoint" arg to azure storage builder.[https://github.com/loco-rs/loco/pull/1317](https://github.com/loco-rs/loco/pull/1317)
* Improve readability and performance by using  map_err in Model. [https://github.com/loco-rs/loco/pull/1311](https://github.com/loco-rs/loco/pull/1311)
### Breaking Changes
In module `loco_rs::auth::jwt` in struct `JWT`, the impl method `generate_token` signature has changed. 
Migration:

Before
```rust
jwt.generate_token(&expiration, pid.clone(), None);
```
After
```rust
jwt.generate_token(expiration, pid.clone(), Map::new());
//                 ^ no "&"                 ^ serde_json::map (doesn't allocate in constructor)
```

## v0.14.1

* Fix: bump shuttle to 0.51.0. [https://github.com/loco-rs/loco/pull/1169](https://github.com/loco-rs/loco/pull/1169)
* Return 422 status code for JSON rejection errors. [https://github.com/loco-rs/loco/pull/1173](https://github.com/loco-rs/loco/pull/1173)
* Address clippy warnings for Rust stable 1.84. [https://github.com/loco-rs/loco/pull/1168](https://github.com/loco-rs/loco/pull/1168)
* Bump shuttle to 0.51.0. [https://github.com/loco-rs/loco/pull/1169](https://github.com/loco-rs/loco/pull/1169)
* Return 422 status code for JSON rejection errors. [https://github.com/loco-rs/loco/pull/1173](https://github.com/loco-rs/loco/pull/1173)
* Return json validation details response. [https://github.com/loco-rs/loco/pull/1174](https://github.com/loco-rs/loco/pull/1174)
* Fix example command after generating schedule. [https://github.com/loco-rs/loco/pull/1176](https://github.com/loco-rs/loco/pull/1176)
* Fixed independent features. [https://github.com/loco-rs/loco/pull/1177](https://github.com/loco-rs/loco/pull/1177)
* Custom response header for redirect. [https://github.com/loco-rs/loco/pull/1186](https://github.com/loco-rs/loco/pull/1186)
* Added run_on_start feature to scheduler. [https://github.com/loco-rs/loco/pull/1184](https://github.com/loco-rs/loco/pull/1184)
* feat: public jwt extractor from non-mutable reference to parts. [https://github.com/loco-rs/loco/pull/1190](https://github.com/loco-rs/loco/pull/1190)


## v0.14

* feat: smart migration generator. you can now generate migration based on naming them for creating a table, adding columns, references, join tables and more. [https://github.com/loco-rs/loco/pull/1086](https://github.com/loco-rs/loco/pull/1086)
* feat: `cargo loco routes` will now pretty-print routes
* fix: guard jwt error behind feature flag. [https://github.com/loco-rs/loco/pull/1032](https://github.com/loco-rs/loco/pull/1032)
* fix: logger file_appender not using the seperated format setting. [https://github.com/loco-rs/loco/pull/1036](https://github.com/loco-rs/loco/pull/1036)
* seed cli command. [https://github.com/loco-rs/loco/pull/1046](https://github.com/loco-rs/loco/pull/1046)
* Updated validator to 0.19. [https://github.com/loco-rs/loco/pull/993](https://github.com/loco-rs/loco/pull/993)
  ### Breaking Changes
  Bump validator to 0.19 in your local `Cargo.toml`
* Testing helpers: simplified function calls + adding html selector. [https://github.com/loco-rs/loco/pull/1047](https://github.com/loco-rs/loco/pull/1047)
  ### Breaking Changes
  #### Updated Import Paths
  The testing module import path has been updated. To adapt your code, update imports from:
  ```rust
  use loco_rs::testing;
  ```
  to:
  ```rust
  use testing::prelude::*;
  ```
  #### Simplified Function Calls
  Function calls within the testing module no longer require the testing:: prefix. Update your code accordingly. For example:
  
  Before:
  ```rust
  let boot = testing::boot_test::<App>().await.unwrap();
  ```

  After:
  ```rust
  let boot = boot_test::<App>().await.unwrap();
  ```
* implement commands to manage background jobs. [https://github.com/loco-rs/loco/pull/1071](https://github.com/loco-rs/loco/pull/1071)
* magic link. [https://github.com/loco-rs/loco/pull/1085](https://github.com/loco-rs/loco/pull/1085)
* infer migration. [https://github.com/loco-rs/loco/pull/1086](https://github.com/loco-rs/loco/pull/1086)
* Remove unnecessary calls to 'register_tasks' functions in scheduler. [https://github.com/loco-rs/loco/pull/1100](https://github.com/loco-rs/loco/pull/1100)
* implement commands to manage background jobs. [https://github.com/loco-rs/loco/pull/1071](https://github.com/loco-rs/loco/pull/1071)
* expose hello_name for SMTP client config. [https://github.com/loco-rs/loco/pull/1057](https://github.com/loco-rs/loco/pull/1057)
* use reqwest with rustls rather than openssl. [https://github.com/loco-rs/loco/pull/1058](https://github.com/loco-rs/loco/pull/1058)
* more flexible config, take more values from ENV. [https://github.com/loco-rs/loco/pull/1058](https://github.com/loco-rs/loco/pull/1058)
* refactor: Use opendal to replace object_store. [https://github.com/loco-rs/loco/pull/897](https://github.com/loco-rs/loco/pull/897)
* allow override loco template. [https://github.com/loco-rs/loco/pull/1102](https://github.com/loco-rs/loco/pull/1102)
* support custom config folder. [https://github.com/loco-rs/loco/pull/1081](https://github.com/loco-rs/loco/pull/1081)
* feat: upgrade to Axum 8. [https://github.com/loco-rs/loco/pull/1130](https://github.com/loco-rs/loco/pull/1130)
* create load config hook. [https://github.com/loco-rs/loco/pull/1143](https://github.com/loco-rs/loco/pull/1143)
* initial impl new migration dsl. [https://github.com/loco-rs/loco/pull/1125](https://github.com/loco-rs/loco/pull/1125)
* allow disable limit_payload middleware. [https://github.com/loco-rs/loco/pull/1113](https://github.com/loco-rs/loco/pull/1113)


## v0.13.2

* static fallback now returns 200 and not 404 [https://github.com/loco-rs/loco/pull/991](https://github.com/loco-rs/loco/pull/991)
* cache system now has expiry [https://github.com/loco-rs/loco/pull/1006](https://github.com/loco-rs/loco/pull/1006)
* fixed: http interface binding [https://github.com/loco-rs/loco/pull/1007](https://github.com/loco-rs/loco/pull/1007)
* JWT claims now editable and public [https://github.com/loco-rs/loco/issues/988](https://github.com/loco-rs/loco/issues/988)
* CORS now not enabled in dev mode to avoid friction [https://github.com/loco-rs/loco/pull/1009](https://github.com/loco-rs/loco/pull/1009)
* fixed: task code generation now injects in all cases [https://github.com/loco-rs/loco/pull/1012](https://github.com/loco-rs/loco/pull/1012)

**BREAKING**
In your `app.rs` add the following injection comment at the bottom:

```rust
fn register_tasks(tasks: &mut Tasks) {
    tasks.register(tasks::user_report::UserReport);
    tasks.register(tasks::seed::SeedData);
    tasks.register(tasks::foo::Foo);
    // tasks-inject (do not remove)
}
```
* fix: seeding now sets autoincrement fields in the relevant DBs [https://github.com/loco-rs/loco/pull/1014](https://github.com/loco-rs/loco/pull/1014)
* fix: avoid generating entities from queue tables when the queue backend is database based [https://github.com/loco-rs/loco/issues/1013](https://github.com/loco-rs/loco/issues/1013)
* removed: channels moved to an initializer [https://github.com/loco-rs/loco/issues/892](https://github.com/loco-rs/loco/issues/892)
**BREAKING**
See how this looks like in [https://github.com/loco-rs/chat-rooms](https://github.com/loco-rs/chat-rooms)

## v0.13.0

* Added SQLite background job support [https://github.com/loco-rs/loco/pull/969](https://github.com/loco-rs/loco/pull/969)
* Added automatic updating of `updated_at` on change [https://github.com/loco-rs/loco/pull/962](https://github.com/loco-rs/loco/pull/962)
* fixed codegen injection point in migrations [https://github.com/loco-rs/loco/pull/952](https://github.com/loco-rs/loco/pull/952)

**NOTE: update your migration listing module like so:**

```rust
// migrations/src/lib.rs
  vec![
      Box::new(m20220101_000001_users::Migration),
      Box::new(m20231103_114510_notes::Migration),
      Box::new(m20240416_071825_roles::Migration),
      Box::new(m20240416_082115_users_roles::Migration),
      // inject-above (do not remove this comment)
  ]
```

Add the comment just before the closing array (`inject-above`)

* Added ability to name references in [https://github.com/loco-rs/loco/pull/955](https://github.com/loco-rs/loco/pull/955):

```sh
$ generate scaffold posts title:string! content:string! written_by:references:users approved_by:references:users
```

* Added hot-reload like experience to Tera templates [https://github.com/loco-rs/loco/issues/977](https://github.com/loco-rs/loco/issues/977), in debug builds only.

**NOTE: update your initializers `after_routes` like so:**

```rust
// src/initializers/view_engine.rs
async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
    #[allow(unused_mut)]
    let mut tera_engine = engines::TeraView::build()?;
    if std::path::Path::new(I18N_DIR).exists() {
        let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
            .shared_resources(Some(&[I18N_SHARED.into()]))
            .customize(|bundle| bundle.set_use_isolating(false))
            .build()
            .map_err(|e| Error::string(&e.to_string()))?;
        #[cfg(debug_assertions)]
        tera_engine
            .tera
            .lock()
            .expect("lock")
            .register_function("t", FluentLoader::new(arc));

        #[cfg(not(debug_assertions))]
        tera_engine
            .tera
            .register_function("t", FluentLoader::new(arc));
        info!("locales loaded");
    }

    Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
}
```

* `loco doctor` now checks for app-specific minimum dependency versions. This should help in upgrades. `doctor` also supports "production only" checks which you can run in production with `loco doctor --production`. This, for example, will check your connections but will not check dependencies. [https://github.com/loco-rs/loco/pull/931](https://github.com/loco-rs/loco/pull/931)
* Use a single loco-rs dep for a whole project. [https://github.com/loco-rs/loco/pull/927](https://github.com/loco-rs/loco/pull/927)
* chore: fix generated testcase. [https://github.com/loco-rs/loco/pull/939](https://github.com/loco-rs/loco/pull/939)
* chore: Correct cargo test message. [https://github.com/loco-rs/loco/pull/938](https://github.com/loco-rs/loco/pull/938)
* Add relevant meta tags for better defaults. [https://github.com/loco-rs/loco/pull/943](https://github.com/loco-rs/loco/pull/943)
* Update cli message with correct command. [https://github.com/loco-rs/loco/pull/942](https://github.com/loco-rs/loco/pull/942)
* remove lazy_static. [https://github.com/loco-rs/loco/pull/941](https://github.com/loco-rs/loco/pull/941)
* change update HTTP verb semantics to put+patch. [https://github.com/loco-rs/loco/pull/919](https://github.com/loco-rs/loco/pull/919)
* Fixed HTML scaffold error. [https://github.com/loco-rs/loco/pull/960](https://github.com/loco-rs/loco/pull/960)
* Scaffolded HTML update method should be POST. [https://github.com/loco-rs/loco/pull/963](https://github.com/loco-rs/loco/pull/963)

## v0.12.0

This release have been primarily about cleanups and simplification.

Please update:

* `loco-rs`
* `loco-cli`

Changes:

* **generators (BREAKING)**: all prefixes in starters (e.g. `/api`) are now _local to each controller_, and generators will be prefix-aware (`--api` generator will add an `/api` prefix to controllers) [https://github.com/loco-rs/loco/pull/818](https://github.com/loco-rs/loco/pull/818)

To migrate, please move prefixes from `app.rs` to each controller you use in `controllers/`, for example in `notes` controller:

```rust
Routes::new()
    .prefix("api/notes")
    .add("/", get(list))
```

* **starters**: removed `.devcontainer` which can now be found in [loco-devcontainer](https://github.com/loco-rs/loco-devcontainer)
* **starters**: removed example `notes` scaffold (model, controllers, etc), and unified `user` and `auth` into a single file: `auth.rs`
* **generators**: `scaffold` generator will now generate a CRUD with `PUT` and `PATCH` semantics for updating an entity [https://github.com/loco-rs/loco/issues/896](https://github.com/loco-rs/loco/issues/896)
* **cleanup**: `loco-extras` was moved out of the repo, but we've incorporated `MultiDB` and `ExtraDB` from `extras` into `loco-rs` [https://github.com/loco-rs/loco/pull/917](https://github.com/loco-rs/loco/pull/917)

* `cargo loco doctor` now checks for minimal required SeaORM CLI version
* **BREAKING** Improved migration generator. If you have an existing migration project, add the following comment indicator to the top of the `vec` statement and right below the opening bracked like so in `migration/src/lib.rs`:
```rust
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // inject-below (do not remove this comment)
```

## v0.11.0


* Upgrade **SeaORM to v1.1.0**
* Added OpenAPI example
* Improve health route [https://github.com/loco-rs/loco/pull/851](https://github.com/loco-rs/loco/pull/851)
* Add good pragmas to Sqlite [https://github.com/loco-rs/loco/pull/848](https://github.com/loco-rs/loco/pull/848)
* Upgrade to rsbuild 1.0. [https://github.com/loco-rs/loco/pull/792](https://github.com/loco-rs/loco/pull/792)
* Implements fmt::Debug to pub structs. [https://github.com/loco-rs/loco/pull/812](https://github.com/loco-rs/loco/pull/812)
* Add num_workers config for sidekiq queue. [https://github.com/loco-rs/loco/pull/823](https://github.com/loco-rs/loco/pull/823)
* Fix some comments in the starters and example code. [https://github.com/loco-rs/loco/pull/824](https://github.com/loco-rs/loco/pull/824)
* Fix Y2038 bug for JWT on 32 bit platforms. [https://github.com/loco-rs/loco/pull/825](https://github.com/loco-rs/loco/pull/825)
* Make App URL in Boot Banner Clickable. [https://github.com/loco-rs/loco/pull/826](https://github.com/loco-rs/loco/pull/826)
* Add `--no-banner` flag to allow disabling the banner display. [https://github.com/loco-rs/loco/pull/839](https://github.com/loco-rs/loco/pull/839)
* add on_shutdown hook. [https://github.com/loco-rs/loco/pull/842](https://github.com/loco-rs/loco/pull/842)


## v0.10.1

* `Format(respond_to): Format` extractor in controller can now be replaced with `respond_to: RespondTo` extractor for less typing.
* When supplying data to views, you can now use `data!` instead of `serde_json::json!` for shorthand.
* Refactor middlewares. [https://github.com/loco-rs/loco/pull/785](https://github.com/loco-rs/loco/pull/785). Middleware selection, configuration, and tweaking is MUCH more powerful and convenient now. You can keep the `middleware:` section empty or remove it now, see more in [the middleware docs](https://loco.rs/docs/the-app/controller/#middleware)
* **NEW (BREAKING)** background worker subsystem is now queue agnostic. Providing for both Redis and Postgres with a change of configuration. This means you can now use a full-Postgres stack to remove Redis as a dependency if you wish. Here are steps to migrate your codebase:

```rust
// in your app.rs, change the worker registration code:

// BEFORE
fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {
    p.register(DownloadWorker::build(ctx)); 
}

// AFTER
async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()>{
    queue.register(DownloadWorker::build(ctx)).await?;
    Ok(())
}

// in your app.rs, replace the `worker` module references.
// REMOVE
worker::{AppWorker, Processor},
// REPLACE WITH
bgworker::{BackgroundWorker, Queue},

// in your workers change the signature, and add the `build` function

// BEFORE
impl worker::Worker<DownloadWorkerArgs> for DownloadWorker {
    async fn perform(&self, args: DownloadWorkerArgs) -> worker::Result<()> {

// AFTER
#[async_trait]
impl BackgroundWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    async fn perform(&self, args: DownloadWorkerArgs) -> Result<()> {
  
// Finally, remove the `AppWorker` trait implementation completely.

// REMOVE
impl worker::AppWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
}
```

Finally, update your `development.yaml` and `test.yaml` with a `kind`:

```yaml
queue:
  kind: Redis  # add this to the existing `queue` section
```


* **UPGRADED (BREAKING)**: `validator` crate was upgraded which require some small tweaks to work with the new API:

```rust
// BEFORE:
#[validate(custom = "validation::is_valid_email")]
pub email: String,

// AFTER:
#[validate(custom (function = "validation::is_valid_email"))]
pub email: String,
```

Then update your `Cargo.toml` to take version `0.18`:

```toml
# update
validator = { version = "0.18" }
```

* **UPGRADED (BREAKING)**: `axum-test` crate was upgraded 
Update your `Cargo.toml` to version `16`:

```toml
# update
axum-test = { version = "16" }
```

## v0.9.0

* Add fallback behavior. [https://github.com/loco-rs/loco/pull/732](https://github.com/loco-rs/loco/pull/732)
* Add Scheduler Feature for Running Cron Jobs. [https://github.com/loco-rs/loco/pull/735](https://github.com/loco-rs/loco/pull/735)
* Add `--html`, `--htmx` and `--api` flags to scaffold CLI command. [https://github.com/loco-rs/loco/pull/749](https://github.com/loco-rs/loco/pull/749)
* Add base template for scaffold generation. [https://github.com/loco-rs/loco/pull/752](https://github.com/loco-rs/loco/pull/752)
* Connect Redis only when the worker is BackgroundQueue. [https://github.com/loco-rs/loco/pull/755](https://github.com/loco-rs/loco/pull/755)
* Add loco doctor --config. [https://github.com/loco-rs/loco/pull/736](https://github.com/loco-rs/loco/pull/736)
* Rename demo: blo -> demo_app. [https://github.com/loco-rs/loco/pull/741](https://github.com/loco-rs/loco/pull/741)


## v0.8.1
* fix: introduce secondary binary for compile-and-run on Windows. [https://github.com/loco-rs/loco/pull/727](https://github.com/loco-rs/loco/pull/727)


## v0.8.0

* Added: loco-cli (`loco new`) now receives options from CLI and/or interactively asks for configuration options such as which asset pipeline, background worker type, or database provider to use.
* Fix: custom queue names now merge with default queues.
* Added `remote_ip` middleware for resolving client remote IP when under a proxy or loadbalancer, similar to the Rails `remote_ip` middleware.
* Added `secure_headers` middleware for setting secure headers by default, similar to how [https://github.com/github/secure_headers](https://github.com/github/secure_headers) works. This is now ON by default to promote security-by-default.
* Added: `money`, `blob` types to entitie generator.

## 0.7.0
* Moving to _timezone aware timestamps_. From now on migrations will generate **timestamps with time zone** by default. Moving to TZ aware timestamps in combination with newly revamped timestamp code generation in SeaORM v1.0.0 finally allows for _seamlessly_ moving between using `sqlite` and `postgres` with minimal or no entities code changes (resolved [this long standing issue](https://github.com/loco-rs/loco/issues/518#issuecomment-2051708319)). TZ aware timestamps also aligns us with how Rails works today (initially Rails had a no-tz timestamps, and today the default is to use timestamps). If not specified the TZ is the server TZ, which is usually UTC, therefore semantically this is almost like a no-tz timestamp.

**A few highlights:**

Generated entities will now always use `DateTimeWithTimeZone` for the default timestamp fields:

```
...
Generating users.rs
    > Column `created_at`: DateTimeWithTimeZone, not_null
    > Column `updated_at`: DateTimeWithTimeZone, not_null
...
```

For better cross database provider compatibility, from now on prefer the `tstz` type instead of just `ts` when using generators (i.e. `cargo loco generate model movie released:tstz`)

* remove eyer lib. [https://github.com/loco-rs/loco/pull/650](https://github.com/loco-rs/loco/pull/650)
  ### Breaking Changes:
     1. Update the Main Function in src/bin/main
   
      Replace the return type of the main function:
   
      **Before:**
      ```rust
      async fn main() -> eyre::Result<()>
      ```
   
      **After:**
      ```rust
      async fn main() -> loco_rs::Result<()>
      ```
   
   
   2. Modify examples/playground.rs
      You need to apply two changes here:
   
        a. Update the Function Signature
        **Before:**
        ```rust
        async fn main() -> eyre::Result<()>
        ```
   
        **After:**
        ```rust
        async fn main() -> loco_rs::Result<()>
        ```
   
        b. Adjust the Context Handling
        **Before:**
        ```rust
        let _ctx = playground::<App>().await.context("playground")?;
        ```
   
        **After:**
        ```rust
        let _ctx = playground::<App>().await?;
        ```
   
   Note, 
   If you are using eyre in your project, you can continue to do so. We have only removed this crate from our base code dependencies.
* Bump rstest crate to 0.21.0. [https://github.com/loco-rs/loco/pull/650](https://github.com/loco-rs/loco/pull/650)
* Bump serial_test crate to 3.1.1. [https://github.com/loco-rs/loco/pull/651](https://github.com/loco-rs/loco/pull/651)
* Bumo object store to create to 0.10.2. [https://github.com/loco-rs/loco/pull/654](https://github.com/loco-rs/loco/pull/654)
* Bump axum crate to 0.7.5. [https://github.com/loco-rs/loco/pull/652](https://github.com/loco-rs/loco/pull/652)
* Add Hooks::before_routes to give user control over initial axum::Router construction. [https://github.com/loco-rs/loco/pull/646](https://github.com/loco-rs/loco/pull/646)
* Support logger file appender. [https://github.com/loco-rs/loco/pull/636](https://github.com/loco-rs/loco/pull/636)
* Response from the template. [https://github.com/loco-rs/loco/pull/682](https://github.com/loco-rs/loco/pull/682)
* Add get_or_insert function to cache layer. [https://github.com/loco-rs/loco/pull/637](https://github.com/loco-rs/loco/pull/637)
* Bump ORM create to 1.0.0. [https://github.com/loco-rs/loco/pull/684](https://github.com/loco-rs/loco/pull/684)


## 0.6.2
* Use Rust-based tooling for SaaS starter frontend. [https://github.com/loco-rs/loco/pull/625](https://github.com/loco-rs/loco/pull/625)
* Default binding to localhost to avoid firewall dialogues during development on macOS. [https://github.com/loco-rs/loco/pull/627](https://github.com/loco-rs/loco/pull/627)
* upgrade sea-orm to 1.0.0 RC 7. [https://github.com/loco-rs/loco/pull/627](https://github.com/loco-rs/loco/pull/639)
* Add a down migration command. [https://github.com/loco-rs/loco/pull/414](https://github.com/loco-rs/loco/pull/414)
* replace create_postgres_database function table_name to db_name. [https://github.com/loco-rs/loco/pull/647](https://github.com/loco-rs/loco/pull/647)

## 0.6.1
 * Upgrade htmx generator to htmx2. [https://github.com/loco-rs/loco/pull/629](https://github.com/loco-rs/loco/pull/629)


## 0.6.0 https://github.com/loco-rs/loco/pull/610
* Bump socketioxide to v0.13.1. [https://github.com/loco-rs/loco/pull/594](https://github.com/loco-rs/loco/pull/594)
* Add CC and BCC fields to the mailers. [https://github.com/loco-rs/loco/pull/599](https://github.com/loco-rs/loco/pull/599)
* Delete reset tokens after use. [https://github.com/loco-rs/loco/pull/602](https://github.com/loco-rs/loco/pull/602)
* Generator html support delete entity. [https://github.com/loco-rs/loco/pull/604](https://github.com/loco-rs/loco/pull/604)
* **Breaking changes** move task args from BTreeMap to struct. [https://github.com/loco-rs/loco/pull/609](https://github.com/loco-rs/loco/pull/609)
  * Change task signature from `async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>)` to `async fn run(&self, _app_context: &AppContext, _vars: &task::Vars) -> Result<()>`
  *  **Breaking changes** change default port to 5150. [https://github.com/loco-rs/loco/pull/611](https://github.com/loco-rs/loco/pull/611)
*  Update shuttle version in deployment generation. [https://github.com/loco-rs/loco/pull/616](https://github.com/loco-rs/loco/pull/616)

## v0.5.0 https://github.com/loco-rs/loco/pull/593

* refactor auth middleware for supporting bearer, cookie and query. [https://github.com/loco-rs/loco/pull/560](https://github.com/loco-rs/loco/pull/560)
* SeaORM upgraded: `rc1` -> `rc4`. [https://github.com/loco-rs/loco/pull/585](https://github.com/loco-rs/loco/pull/585)
* Adding Cache to app content. [https://github.com/loco-rs/loco/pull/570](https://github.com/loco-rs/loco/pull/570)
* Apply a layer to a specific handler using `layer` method. [https://github.com/loco-rs/loco/pull/554](https://github.com/loco-rs/loco/pull/554)
* Add the debug macro to the templates to improve the errors. [https://github.com/loco-rs/loco/pull/547](https://github.com/loco-rs/loco/pull/547)
* Opentelemetry initializer. [https://github.com/loco-rs/loco/pull/531](https://github.com/loco-rs/loco/pull/531)
* Refactor auth middleware for supporting bearer, cookie and query [https://github.com/loco-rs/loco/pull/560](https://github.com/loco-rs/loco/pull/560)
* Add redirect response [https://github.com/loco-rs/loco/pull/563](https://github.com/loco-rs/loco/pull/563)
* **Breaking changes** Adding a custom claims `Option<serde_json::Value>` to the `UserClaims` struct (type changed). [https://github.com/loco-rs/loco/pull/578](https://github.com/loco-rs/loco/pull/578)
* **Breaking changes** Refactored DSL and Pagination: namespace changes. [https://github.com/loco-rs/loco/pull/566](https://github.com/loco-rs/loco/pull/566)
  * Replaced `model::query::dsl::` with `model::query`.
  * Replaced `model::query::exec::paginate` with `model::query::paginate`.
  * Updated the `PaginatedResponse` struct. Refer to its usage example [here](https://github.com/loco-rs/loco/blob/master/examples/demo/src/views/notes.rs#L29).
* **Breaking changes** When introducing the Cache system which is much more flexible than having just Redis, we now call the 'redis' member simply a 'queue' which indicates it should be used only for the internal queue and not as a general purpose cache. In the application configuration setting `redis`, change to `queue`. [https://github.com/loco-rs/loco/pull/590](https://github.com/loco-rs/loco/pull/590)
```yaml
# before:
redis:
# after:
queue:
```
* **Breaking changes** We have made a few parts of the context pluggable, such as the `storage` and new `cache` subsystems, this is why we decided to let you configure the context entirely before starting up your app. As a result, if you have a storage building hook code it should move to `after_context`, see example [here](https://github.com/loco-rs/loco/pull/570/files#diff-5534e8826fb82e5c7f2587d270a51b48009341e79889d1504e6b63b2f0b652bdR83). [https://github.com/loco-rs/loco/pull/570](https://github.com/loco-rs/loco/pull/570)

## v0.4.0

* Refactored model validation for better developer experience. Added a few traits and structs to `loco::prelude` for a smoother import story. Introducing `Validatable`:

```rust
impl Validatable for super::_entities::users::ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
            email: self.email.as_ref().to_owned(),
        })
    }
}

// now you can call `user.validate()` freely
```

* Refactored type field mapping to be centralized. Now model, scaffold share the same field mapping, so no more gaps like [https://github.com/loco-rs/loco/issues/513](https://github.com/loco-rs/loco/issues/513) (e.g. when calling `loco generate model title:string` the ability to map `string` into something useful in the code generation side)
**NOTE** the `_integer` class of types are now just `_int`, e.g. `big_int`, so that it correlate with the `int` field name in a better way

* Adding to to quiery dsl `is_in` and `is_not_in`. [https://github.com/loco-rs/loco/pull/507](https://github.com/loco-rs/loco/pull/507)
* Added: in your configuration you can now use an `initializers:` section for initializer specific settings

  ```yaml
  # Initializers Configuration
  initializers:
  # oauth2:
  #   authorization_code: # Authorization code grant type
  #     - client_identifier: google # Identifier for the OAuth2 provider. Replace 'google' with your provider's name if different, must be unique within the oauth2 config.
  #       ... other fields
  ```

* Docs: fix schema data types mapping. [https://github.com/loco-rs/loco/pull/506](https://github.com/loco-rs/loco/pull/506)
* Let Result accept other errors. [https://github.com/loco-rs/loco/pull/505](https://github.com/loco-rs/loco/pull/505)
* Allow trailing slashes in URIs by adding the NormalizePathLayer. [https://github.com/loco-rs/loco/pull/481](https://github.com/loco-rs/loco/pull/481)
* **BREAKING** Move from `Result<impl IntoResponse>` to `Result<Response>`. This enables much greater flexibility building APIs, where with `Result<Response>` you mix and match response types based on custom logic (returning JSON and HTML/String in the same route).
* **Added**: mime responders similar to `respond_to` in Rails:

1. Use the `Format` extractor
2. Match on `respond_to`
3. Create different content for different response formats

The following route will always return JSON, unless explicitly asked for HTML with a
`Content-Type: text/html` (or `Accept: `) header:

```rust
pub async fn get_one(
    Format(respond_to): Format,
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    match respond_to {
        RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
        _ => format::json(item),
    }
}
```

## 0.3.2

* Redisgin pagination. [https://github.com/loco-rs/loco/pull/463](https://github.com/loco-rs/loco/pull/463)
* Wrap seaorm query and condition for common use cases. [https://github.com/loco-rs/loco/pull/463](https://github.com/loco-rs/loco/pull/463)
* Adding to loco-extras initializer for extra or multiple db. [https://github.com/loco-rs/loco/pull/471](https://github.com/loco-rs/loco/pull/471)
* Scaffold now supporting different templates such as API,HTML or htmx, this future is in beta.[https://github.com/loco-rs/loco/pull/474](https://github.com/loco-rs/loco/pull/474)
* Fix generatore fields types + adding tests. [https://github.com/loco-rs/loco/pull/459](https://github.com/loco-rs/loco/pull/459)
* Fix channel cors. [https://github.com/loco-rs/loco/pull/430](https://github.com/loco-rs/loco/pull/430)
* Improve auth controller compatibility with frontend [https://github.com/loco-rs/loco/pull/472](https://github.com/loco-rs/loco/pull/472)


## 0.3.1

* **Breaking changes** Upgrade sea-orm to v1.0.0-rc.1. [https://github.com/loco-rs/loco/pull/420](https://github.com/loco-rs/loco/pull/420)
  Needs to update `sea-orm` crate to use `v1.0.0-rc.1` version.
* Implemented file upload support with versatile strategies. [https://github.com/loco-rs/loco/pull/423](https://github.com/loco-rs/loco/pull/423)
* Create a `loco_extra` crate to share common basic implementations. [https://github.com/loco-rs/loco/pull/425](https://github.com/loco-rs/loco/pull/425)
* Update shuttle deployment template to 0.38. [https://github.com/loco-rs/loco/pull/422](https://github.com/loco-rs/loco/pull/422)
* Enhancement: Move the Serve to Hook flow with the ability to override default serve settings. [https://github.com/loco-rs/loco/pull/418](https://github.com/loco-rs/loco/pull/418)
* Avoid cloning sea_query::ColumnDef. [https://github.com/loco-rs/loco/pull/415](https://github.com/loco-rs/loco/pull/415)
* Allow required UUID type in a scaffold. [https://github.com/loco-rs/loco/pull/408](https://github.com/loco-rs/loco/pull/408)
* Cover `SqlxMySqlPoolConnection` in db.rs. [https://github.com/loco-rs/loco/pull/411](https://github.com/loco-rs/loco/pull/411)
* Update worker docs and change default worker mode. [https://github.com/loco-rs/loco/pull/412](https://github.com/loco-rs/loco/pull/412)
* Added server-side view generation through a new `ViewEngine` infrastructure and `Tera` server-side templates: [https://github.com/loco-rs/loco/pull/389](https://github.com/loco-rs/loco/pull/389)
* Added `generate model --migration-only` [https://github.com/loco-rs/loco/issues/400](https://github.com/loco-rs/loco/issues/400)
* Add JSON to scaffold gen. [https://github.com/loco-rs/loco/pull/396](https://github.com/loco-rs/loco/pull/396)
* Add --binding(-b) and --port(-b) to `cargo loco start`.[https://github.com/loco-rs/loco/pull/402](https://github.com/loco-rs/loco/pull/402)

## 0.2.3

* Add: support for [pre-compressed assets](https://github.com/loco-rs/loco/pull/370/files).
* Added: Support socket channels, see working example [here](https://github.com/loco-rs/chat-rooms). [https://github.com/loco-rs/loco/pull/380](https://github.com/loco-rs/loco/pull/380)
* refactor: optimize checking permissions on Postgres. [9416c](https://github.com/loco-rs/loco/commit/9416c5db85a27e3d30471374effec3fe88bf80a2)
* Added: E2E db. [https://github.com/loco-rs/loco/pull/371](https://github.com/loco-rs/loco/pull/371)

## v0.2.2
* fix: public fields in mailer-op. [e51b7e](https://github.com/loco-rs/loco/commit/e51b7e64e7667c519451ac8a8bea574b2c5d4403)
* fix: handle missing db permissions. [e51b7e](https://github.com/loco-rs/loco/commit/e51b7e64e7667c519451ac8a8bea574b2c5d4403)

## v0.2.1
* enable compression for CompressionLayer, not etag. [https://github.com/loco-rs/loco/pull/356](https://github.com/loco-rs/loco/pull/356)
* Fix nullable JSONB column schema definition. [https://github.com/loco-rs/loco/pull/357](https://github.com/loco-rs/loco/pull/357)

## v0.2.0

* Add: Loco now has Initializers ([see the docs](https://loco.rs/docs/the-app/initializers/)). Initializers help you integrate infra into your app in a seamless way, as well as share pieces of setup code between your projects
* Add: an `init_logger` hook in `src/app.rs` for those who want to take ownership of their logging and tracing stack.
* Add: Return a JSON schema when payload json could not serialize to a struct. [https://github.com/loco-rs/loco/pull/343](https://github.com/loco-rs/loco/pull/343)
* Init logger in cli.rs. [https://github.com/loco-rs/loco/pull/338](https://github.com/loco-rs/loco/pull/338)
* Add: return JSON schema in panic HTTP layer. [https://github.com/loco-rs/loco/pull/336](https://github.com/loco-rs/loco/pull/336)
* Add: JSON field support in model generation. [https://github.com/loco-rs/loco/pull/327](https://github.com/loco-rs/loco/pull/327) [https://github.com/loco-rs/loco/pull/332](https://github.com/loco-rs/loco/pull/332)
* Add: float support in model generation. [https://github.com/loco-rs/loco/pull/317](https://github.com/loco-rs/loco/pull/317) 
* Fix: conflicting idx definition on M:M migration. [https://github.com/loco-rs/loco/issues/311](https://github.com/loco-rs/loco/issues/311)
* Add: **Breaking changes** Supply `AppContext` to `routes` Hook. Migration steps in `src/app.rs`:

```rust
// src/app.rs: add app context to routes function
impl Hooks for App {
  ...
  fn routes(_ctx: &AppContext) -> AppRoutes;
  ...
}
```

* Add: **Breaking changes** change parameter type from `&str` to `&Environment` in `src/app.rs`

```rust
// src/app.rs: change parameter type for `environment` from `&str` to `&Environment`
impl Hooks for App {
    ...
    async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult> {
        create_app::<Self>(mode, environment).await
    }
    ...
```

* Added: setting cookies:

```rust
format::render()
    .cookies(&[
        cookie::Cookie::new("foo", "bar"),
        cookie::Cookie::new("baz", "qux"),
    ])?
    .etag("foobar")?
    .json(notes)
```


## v0.1.9

* Adding [pagination](https://loco.rs/docs/the-app/pagination/) on Models. [https://github.com/loco-rs/loco/pull/238](https://github.com/loco-rs/loco/pull/238)
* Adding compression middleware. [https://github.com/loco-rs/loco/pull/205](https://github.com/loco-rs/loco/pull/205)
  Added support for [compression middleware](https://docs.rs/tower-http/0.5.0/tower_http/compression/index.html).
  usage:

```yaml
middlewares:
  compression:
    enable: true
```
* Create a new Database from the CLI. [https://github.com/loco-rs/loco/pull/223](https://github.com/loco-rs/loco/pull/223)
* Validate if seaorm CLI is installed before running `cargo loco db entities` and show a better error to the user. [https://github.com/loco-rs/loco/pull/212](https://github.com/loco-rs/loco/pull/212)
* Adding to `saas and `rest-api` starters a redis and DB in GitHub action workflow to allow users work with github action out of the box. [https://github.com/loco-rs/loco/pull/215](https://github.com/loco-rs/loco/pull/215)
* Adding the app name and the environment to the DB name when creating a new starter. [https://github.com/loco-rs/loco/pull/216](https://github.com/loco-rs/loco/pull/216)
* Fix generator when users adding a `created_at` or `update_at` fields. [https://github.com/loco-rs/loco/pull/214](https://github.com/loco-rs/loco/pull/214)
* Add: `format::render` which allows a builder-like formatting, including setting etag and ad-hoc headers
* Add: Etag middleware, enabled by default in starter projects. Once you set an Etag it will check for cache headers and return `304` if needed. To enable etag in your existing project:

```yaml
#...
  middlewares:
    etag:
      enable: true
```

usage:
```rust
  format::render()
      .etag("foobar")?
      .json(Entity::find().all(&ctx.db).await?)
```


#### Authentication: Added API Token Authentication!

* See [https://github.com/loco-rs/loco/pull/217](https://github.com/loco-rs/loco/pull/217)
Now when you generate a `saas starter` or `rest api` starter you will get additional authentication methods for free:

* Added: authentication added -- **api authentication** where each user has an API token in the schema, and you can authenticate with `Bearer` against that user.
* Added: authentication added -- `JWTWithUser` extractor, which is a convenience for resolving the authenticated JWT claims into a current user from database

**migrating an existing codebase**

Add the following to your generated `src/models/user.rs`:

```rust
#[async_trait]
impl Authenticable for super::_entities::users::Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::ApiKey.eq(api_key))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        super::_entities::users::Model::find_by_pid(db, claims_key).await
    }
}
```

Update imports in this file to include `model::Authenticable`:

```rust
use loco_rs::{
    auth, hash,
    model::{Authenticable, ModelError, ModelResult},
    validation,
    validator::Validate,
};
```

  
## v0.1.8

* Added: `loco version` for getting an operable version string containing logical crate version and git SHA if available: `0.3.0 (<git sha>)`

To migrate to this behavior from earlier versions, it requires adding the following to your `app.rs` app hooks:

```rust
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }
```

Reminder: `loco --version` will give you the current Loco framework which your app was built against and `loco version` gives you your app version.
 
* Added: `loco generate migration` for adding ad-hoc migrations
* Added: added support in model generator for many-to-many link table generation via `loco generate model --link`
* Docs: added Migration section, added relations documentation 1:M, M:M
* Adding .devcontainer to starter projects [https://github.com/loco-rs/loco/issues/170](https://github.com/loco-rs/loco/issues/170)
* **Braking changes**: Adding `Hooks::boot` application. Migration steps:
    ```rust
    // Load boot::{create_app, BootResult, StartMode} from loco_rs lib
    // Load migration: use migration::Migrator; Only when using DB
    // Adding boot hook with the following code
    impl Hooks for App {
      ...
      async fn boot(mode: StartMode, environment: &str) -> Result<BootResult> {
        // With DB:
        create_app::<Self, Migrator>(mode, environment).await
        // Without DB:
        create_app::<Self>(mode, environment).await
      }
      ...
    }
    ```
  
## v0.1.7
* Added pretty backtraces [https://github.com/loco-rs/loco/issues/41](https://github.com/loco-rs/loco/issues/41)
* adding tests for note requests [https://github.com/loco-rs/loco/pull/156](https://github.com/loco-rs/loco/pull/156)
* Define the min rust version the loco can run [https://github.com/loco-rs/loco/pull/164](https://github.com/loco-rs/loco/pull/164)
* Added `cargo loco doctor` cli command for validate and diagnose configurations. [https://github.com/loco-rs/loco/pull/145](https://github.com/loco-rs/loco/pull/145)
* Added ability to specify `settings:` in config files, which are available in context
* Adding compilation mode in the banner. [https://github.com/loco-rs/loco/pull/127](https://github.com/loco-rs/loco/pull/127)
* Support shuttle deployment generator. [https://github.com/loco-rs/loco/pull/124](https://github.com/loco-rs/loco/pull/124)
* Adding a static asset middleware which allows to serve static folder/data. Enable this section in config. [https://github.com/loco-rs/loco/pull/134](https://github.com/loco-rs/loco/pull/134)
  ```yaml
   static:
      enable: true
      # ensure that both the folder.path and fallback file path are existence.
      must_exist: true
      folder: 
        uri: "/assets"
        path: "frontend/dist"        
      fallback: "frontend/dist/index.html" 
  ```
* fix: `loco generate request` test template. [https://github.com/loco-rs/loco/pull/133](https://github.com/loco-rs/loco/pull/133)
* Improve docker deployment generator. [https://github.com/loco-rs/loco/pull/131](https://github.com/loco-rs/loco/pull/131)

## v0.1.6

* refactor: local settings are now `<env>.local.yaml` and available for all environments, for example you can add a local `test.local.yaml` and `development.local.yaml`
* refactor: removed `config-rs` and now doing config loading by ourselves.
* fix: email template rendering will not escape URLs
* Config with variables: It is now possible to use [tera](https://keats.github.io/tera) templates in config YAML files

Example of pulling a port from environment:

```yaml
server:
  port: {{ get_env(name="NODE_PORT", default=5150) }}
```

It is possible to use any `tera` templating constructs such as loops, conditionals, etc. inside YAML configuration files.

* Mailer: expose `stub` in non-test

* `Hooks::before_run` with a default blank implementation. You can now code some custom loading of resources or other things before the app runs
* an LLM inference example, text generation in Rust, using an API (`examples/inference`)
* Loco starters version & create release script [https://github.com/loco-rs/loco/pull/110](https://github.com/loco-rs/loco/pull/110)
* Configure Cors middleware [https://github.com/loco-rs/loco/pull/114](https://github.com/loco-rs/loco/pull/114)
* `Hooks::after_routes` Invoke this function after the Loco routers have been constructed. This function enables you to configure custom Axum logics, such as layers, that are compatible with Axum. [https://github.com/loco-rs/loco/pull/114](https://github.com/loco-rs/loco/pull/114)
* Adding docker deployment generator [https://github.com/loco-rs/loco/pull/119](https://github.com/loco-rs/loco/pull/119)

DOCS:
* Remove duplicated docs in auth section
* FAQ docs: [https://github.com/loco-rs/loco/pull/116](https://github.com/loco-rs/loco/pull/116)

ENHANCEMENTS:
* Remove unused libs: [https://github.com/loco-rs/loco/pull/106](https://github.com/loco-rs/loco/pull/106)
* turn off default features in tokio [https://github.com/loco-rs/loco/pull/118](https://github.com/loco-rs/loco/pull/118)

## 0.1.5

NEW FEATURES
* `format:html` [https://github.com/loco-rs/loco/issues/74](https://github.com/loco-rs/loco/issues/74)
* Create a stateless HTML starter [https://github.com/loco-rs/loco/pull/100](https://github.com/loco-rs/loco/pull/100)
* Added worker generator + adding a way to test workers [https://github.com/loco-rs/loco/pull/92](https://github.com/loco-rs/loco/pull/92)

ENHANCEMENTS:
* CI: allows cargo cli run on fork prs [https://github.com/loco-rs/loco/pull/96](https://github.com/loco-rs/loco/pull/96)

