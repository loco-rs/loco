# Changelog

## vNext
* refactor auth middleware for supporting bearer, cookie and query [https://github.com/loco-rs/loco/pull/560](https://github.com/loco-rs/loco/pull/560)

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
  port: {{ get_env(name="NODE_PORT", default=3000) }}
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

