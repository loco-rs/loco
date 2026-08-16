---
title: Generators & field types
description: Every `cargo loco generate <kind>` component, its exact CLI syntax and output files, plus the complete field-type mini-language used by model/migration/scaffold generators.
sidebar:
  order: 3
---

`cargo loco generate <kind>` (alias `cargo loco g <kind>`) scaffolds application code from templates baked into the `loco-gen` crate. The `generate` subcommand itself is compiled only under `#[cfg(debug_assertions)]` (`src/cli.rs:140`) — it is available in ordinary (dev/debug) builds but is compiled out of `--release` binaries. The kinds that touch the database (`model`, `migration`, `scaffold`) are additionally gated behind the `with-db` Cargo feature (on by default) — see [feature flags](/docs/reference/feature-flags).

This page is the exhaustive dictionary of generator kinds and the field-type mini-language (`name:type`) they all share. It transcribes `loco-gen/src/lib.rs` (the `Component` enum), `loco-gen/src/column.rs` (the field-type/column model), `loco-gen/src/infer.rs` (naming/inflection conventions), and `src/cli.rs` (the CLI surface), re-verified against `HEAD`.

## Generator kinds

`Component` enum: `loco-gen/src/lib.rs:237`. CLI subcommand enum `ComponentArg`: `src/cli.rs:175` (also gated `#[cfg(debug_assertions)]`).

| Kind | CLI syntax | Feature gate | Notes |
|---|---|---|---|
| **model** | `cargo loco generate model <name> [field:type ...] [--without-tz]` | `with-db` | Creates a Sea-ORM entity + model file + migration + tests. `lib.rs:239`, `cli.rs:197` |
| **migration** | `cargo loco generate migration <name> [field:type ...] [--without-tz]` | `with-db` | Standalone migration file; no model/entity. Name-based operation inference (create/add/remove/join) — see [Migration-name inference](#migration-name-inference). `lib.rs:250`, `cli.rs:250` |
| **scaffold** | `cargo loco generate scaffold <name> [field:type ...] [--without-tz] [--no-auth]` | `with-db` | Full CRUD: entity, migration, DTOs, controller, routes, and a model test — plus typed React hooks/pages when the app has a `frontend/`. Adaptive, no kind flag. `lib.rs:102`, `cli.rs:268` |
| **controller** | `cargo loco generate controller <name> [action ...] [--auth]` | none | Controller (JSON API) + routes + a request test only — no model/migration. `lib.rs:117`, `cli.rs:299` |
| **task** | `cargo loco generate task <name>` | none | One-off/CLI task stub, registered in `src/tasks/mod.rs`. `lib.rs:284` |
| **scheduler** | `cargo loco generate scheduler` | none | Writes `config/scheduler.yaml`. `lib.rs:288` |
| **worker** | `cargo loco generate worker <name>` | none | Background worker stub in `src/workers/`, registered in `src/workers/mod.rs`. `lib.rs:289` |
| **mailer** | `cargo loco generate mailer <name>` | none | Mailer struct in `src/mailers/<name>.rs` + embedded `welcome/{subject,html,text}.t` templates. `lib.rs:293` |
| **data** | `cargo loco generate data <name>` | none | Data-loader struct + a `data/<name>/data.json` static file. `lib.rs:297` |
| **deployment** | `cargo loco generate deployment <docker\|nginx\|lambda>` | none | `kind` is a **positional** value, not a `--kind` flag (see [Deployment](#deployment)). |
| **override** | `cargo loco generate override [template_path] [--info]` | none | Copies built-in templates into the app's `.loco-templates/` so you can customize them. `cli.rs:373` |

### Model, migration, scaffold

All three take `name` and a list of `field:type` pairs (the [field-type mini-language](#field-type-mini-language) below), and accept `--without-tz` to omit the `created_at`/`updated_at` timestamp columns. `created_at`, `updated_at`, `create_at`, `update_at` field names are silently skipped if you pass them explicitly (`IGNORE_FIELDS`, `loco-gen/src/model.rs:16`) — they're generated automatically.

```bash
# empty model
cargo loco generate model posts

# model with fields
cargo loco generate model posts title:string! content:text

# model with a belongs-to reference (adds a `director_id` FK column on `movies`)
cargo loco generate model movies long_title:string director:references award:references:prize_id

# migration adding columns to an existing table
cargo loco generate migration AddNameAndAgeToUsers name:string age:int

# scaffold (model + DTOs + controller; adds React hooks/pages if the app has a frontend/)
cargo loco generate scaffold posts title:string! user:references
```

After generating a `migration`, apply it and regenerate entities: `cargo loco db migrate && cargo loco db entities`.

### Scaffold / controller kind

**There is no kind flag.** The 1.0 generators are adaptive: `controller` always generates a JSON API controller, and `scaffold` generates the JSON API plus — when the app has a `frontend/` (a clientside React SPA) — typed React Query hooks and pages for the resource. Headless apps get the backend only. Scaffold detects this from `frontend/src/routes.tsx` (`src/cli.rs`, `Component::Scaffold { frontend }`). For what the frontend half generates and how the TypeScript types stay in sync with your Rust DTOs, see [Build a typed React SPA](/docs/how-to/build-a-spa).

### Authentication on generated routes

A **scaffold is authenticated by default**: all five CRUD handlers take an `auth::JWT` extractor, so an anonymous request answers `401 Unauthorized`. This is deliberate — a scaffolded resource is backed by a real table, and shipping it open by accident is the more expensive mistake. Pass **`--no-auth`** to generate the same controller with public routes:

```bash
# authenticated (default) — requires `Authorization: Bearer <token>`
cargo loco generate scaffold posts title:string!

# public
cargo loco generate scaffold posts title:string! --no-auth
```

The authenticated scaffold prints a one-line reminder that its routes require a JWT, and points at `--no-auth`; `--no-auth` itself prints only the plain "controller was added" line (the note sits inside an `{% if auth %}` in `scaffold/api/controller.t:3`). Either way the React frontend half is unchanged: the SPA sends its bearer token when it has one, and a public API ignores it.

A generated **controller is public by default** — it has no model behind it yet, so there is nothing to protect until you write the handler bodies. **`--auth`** is the opt-in mirror, adding the same `auth::JWT` extractor to every handler (and generating a request test that asserts the route rejects anonymous callers):

```bash
cargo loco generate controller posts --auth
```

To add or remove auth after generating, add or delete the `_auth: auth::JWT,` argument on the handlers you care about — nothing else in the controller depends on it. For how the token is issued and where it is read from, see [JWT authentication](/docs/how-to/jwt-auth) and [JWT locations](/docs/how-to/jwt-locations).

The pre-1.0 `--api` / `--html` / `--htmx` flags (and `-k/--kind`, plus the `ScaffoldKind` enum) were removed with the adaptive rebuild. For backward compatibility the generators still **accept** `--api` (a no-op — it's the headless default) and `--html`/`--htmx` (which error with a pointer to the React SPA frontend that replaced server-rendered views), so existing tutorials and scripts don't fail with a clap `unexpected argument` error (`warn_legacy_scaffold_kind`, `src/cli.rs`).

### Deployment

`DeploymentKind` in `loco-gen` carries generator data (`loco-gen/src/lib.rs:57-75`):

```rust
pub enum DeploymentKind {
    Docker { copy_paths: Vec<PathBuf>, is_client_side_rendering: bool },
    Nginx { host: String, port: i32 },
    Lambda { db: bool, include_paths: Vec<PathBuf> },
}
```

but the **CLI-facing** enum (`src/cli.rs:568-573`) is a plain `clap::ValueEnum { Docker, Nginx, Lambda }` taken as a positional argument — every payload field (`copy_paths`, `is_client_side_rendering`, `host`, `port`, and Lambda's `db` + `include_paths`) is derived from the app's own `config/*.yaml` and filesystem at generation time, not passed on the command line:

```bash
cargo loco generate deployment docker   # writes Dockerfile, .dockerignore
cargo loco generate deployment nginx    # writes nginx/default.conf
cargo loco generate deployment lambda   # writes src/bin/lambda.rs, adds lambda_http
```

### Override

Copies a built-in `.t` template (or a whole folder) into the local `.loco-templates/` directory (`DEFAULT_LOCAL_TEMPLATE`, `loco-gen/src/template.rs:8`) so subsequent generation runs use your copy instead of the built-in one. Delete the local copy to revert to the built-in template.

```bash
# list all overridable templates
cargo loco generate override

# override one file
cargo loco generate override scaffold/api/controller.t

# override every template under a folder
cargo loco generate override scaffold/frontend

# preview what --info would show for a folder, without copying
cargo loco generate override scaffold/api --info

# override everything
cargo loco generate override .
```

## Field-type mini-language

Every `field:type` argument to `model`/`migration`/`scaffold` is resolved in `loco-gen/src/column.rs` — the `parse_column` function and the `ScalarType` enum. Transcribed in full below (re-verified against `HEAD`).

**Suffix convention:** no suffix = nullable `Option<T>`; **`!`** = required (non-null); **`^`** = unique (implies non-null). Not every base type has all three variants — `bool`, `tstz`, and `json` have no `^` (unique) form.

Writing both (`string!^`, `string^!`) is accepted and means the same as `^` alone, since `^` already implies non-null. On a parametrized type either flag may also ride on the base name ahead of the parameters — `decimal_len!:8:24` and `decimal_len:8:24!` are the same column.

**1.0 change:** `int` now maps to **`i64` / `BIGINT`** (`big_integer`), matching the framework's i64 primary keys. Pre-1.0, `int` was `i32`. `unsigned` is an alias of `big_unsigned` (also i64). Use `small_int`/`small_unsigned` if you specifically need 16-bit columns.

| `type` (suffix variants) | Rust type | `ColType` variant | Arity |
|---|---|---|---|
| `uuid` / `uuid!` / `uuid^` | `Option<Uuid>` / `Uuid` / `Uuid` | `UuidNull` / `Uuid` / `UuidUniq` | — |
| `string` / `string!` / `string^` | `Option<String>` / `String` / `String` | `StringNull` / `String` / `StringUniq` | — |
| `text` / `text!` / `text^` | `Option<String>` / `String` / `String` | `TextNull` / `Text` / `TextUniq` | — |
| `small_int` / `!` / `^` | `Option<i16>` / `i16` / `i16` | `SmallIntegerNull` / `SmallInteger` / `SmallIntegerUniq` | — |
| `small_unsigned` / `!` / `^` | `Option<i16>` / `i16` / `i16` | `SmallIntegerNull` / `SmallInteger` / `SmallIntegerUniq` — same `ColType` as `small_int`, see below | — |
| `int` / `!` / `^` **(⚠ i64, was i32 pre-1.0)** | `Option<i64>` / `i64` / `i64` | `BigIntegerNull` / `BigInteger` / `BigIntegerUniq` | — |
| `big_int` / `!` / `^` (alias of `int`) | `Option<i64>` / `i64` / `i64` | `BigIntegerNull` / `BigInteger` / `BigIntegerUniq` | — |
| `unsigned` / `!` / `^` (alias of `big_unsigned`) | `Option<i64>` / `i64` / `i64` | `BigUnsignedNull` / `BigUnsigned` / `BigUnsignedUniq` | — |
| `big_unsigned` / `!` / `^` | `Option<i64>` / `i64` / `i64` | `BigUnsignedNull` / `BigUnsigned` / `BigUnsignedUniq` | — |
| `float` / `!` / `^` | `Option<f32>` / `f32` / `f32` | `FloatNull` / `Float` / `FloatUniq` | — |
| `double` / `!` / `^` | `Option<f64>` / `f64` / `f64` | `DoubleNull` / `Double` / `DoubleUniq` | — |
| `decimal` / `!` / `^` | `Option<Decimal>` / `Decimal` / `Decimal` | `DecimalNull` / `Decimal` / `DecimalUniq` | — |
| `decimal_len` / `!` / `^` | `Option<Decimal>` / `Decimal` / `Decimal` | `DecimalLenNull` / `DecimalLen` / `DecimalLenUniq` | **2** (precision, scale) |
| `bool` / `!` (no `^`) | `Option<bool>` / `bool` | `BooleanNull` / `Boolean` | — |
| `tstz` / `!` (no `^`) | `Option<DateTimeWithTimeZone>` / `DateTimeWithTimeZone` | `TimestampWithTimeZoneNull` / `TimestampWithTimeZone` | — |
| `date` / `!` / `^` | `Option<Date>` / `Date` / `Date` | `DateNull` / `Date` / `DateUniq` | — |
| `time` / `!` / `^` | `Option<Time>` / `Time` / `Time` | `TimeNull` / `Time` / `TimeUniq` | — |
| `date_time` / `!` / `^` | `Option<DateTime>` / `DateTime` / `DateTime` | `DateTimeNull` / `DateTime` / `DateTimeUniq` | — |
| `json` / `!` (no `^`) | `Option<serde_json::Value>` / `serde_json::Value` | `JsonNull` / `Json` | — |
| `jsonb` / `!` / `^` | `Option<serde_json::Value>` / `serde_json::Value` / `serde_json::Value` | `JsonBinaryNull` / `JsonBinary` / `JsonBinaryUniq` | — |
| `blob` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `BlobNull` / `Blob` / `BlobUniq` | — |
| `money` / `!` / `^` | `Option<Decimal>` / `Decimal` / `Decimal` | `MoneyNull` / `Money` / `MoneyUniq` | — |
| `binary_len` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `BinaryLenNull` / `BinaryLen` / `BinaryLenUniq` | **1** (length) |
| `var_binary` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `VarBinaryNull` / `VarBinary` / `VarBinaryUniq` | **1** (length) |
| `array` / `!` / `^` | `Option<Vec<T>>` (see below) | `array_null` / `array` / `array_uniq` (generator emits `ColType::array(ArrayColType::…)` etc.) | **1** (element type) |
| `enum` / `!` / `^` | `Option<T>` / `T` / `T`, where `T` is the PascalCase singular of the *column* name (`status` → `Status`) | `StringNull` / `String` / `StringUniq` — stored as a string column | **1** (comma-separated values, e.g. `status:enum:draft,published`) |

`decimal`, `money`, and `decimal_len` all resolve to the same Rust type (`rust_decimal::Decimal`); the `ColType` distinguishes the SQL representation.

`small_unsigned` deliberately emits the **signed** `SmallInteger*` `ColType` rather than sea-orm's `SmallUnsigned` (`loco-gen/src/column.rs:465`). Neither SQLite nor Postgres has native unsigned integers, and `SmallUnsigned` round-trips as `i16` on SQLite but `i32` on Postgres — so the generated model and DTO would not compile against Postgres. `i16` matches the DTO on both backends.

### Enums

`enum` takes its values as a single comma-separated parameter: `status:enum:draft,published,archived`. The column itself is a plain string in the database. The scaffold generates a real Rust enum next to the DTO — named after the column (`status` → `Status`), one variant per value, `#[serde(rename_all = "snake_case")]`, with a `From<String>` impl and a ts-rs export for the frontend — and the scaffolded form renders the field as a `<select>` over those values.

### Arrays

`array`/`array!`/`array^` take one parameter — the element type — written as a second colon segment: `tags:array:string`, `scores:array!:int`. Valid element types (per `array_inner_from_name` in `loco-gen/src/column.rs`) are `string`, `int`, `big_int`, `float`, `double`, `bool`, generating `Option<Vec<T>>` where `T` is:

| element | Rust `T` |
|---|---|
| `string` | `String` |
| `int` | `i64` |
| `big_int` | `i64` |
| `float` | `f32` |
| `double` | `f64` |
| `bool` | `bool` |

**Note:** array element types are consistent with the scalars in 1.0 — `array:int` generates a 64-bit `BigInt` array (element type `i64`), matching the scalar `int` → `i64` change, per `array_col_type_name` in `loco-gen/src/column.rs` (`ScalarType::Int | ScalarType::BigInt => "BigInt"`). A `column.rs` unit test pins this, asserting `array:big_int!` → `array(ArrayColType::BigInt)`.

### References (belongs-to foreign keys)

A field typed `references` (not in the table above — handled separately in `loco-gen/src/infer.rs:29-54`) generates a belongs-to foreign-key column instead of a regular column:

| Syntax | Meaning |
|---|---|
| `name:references` | Required FK to the `names` table, column `name_id` |
| `name:references:custom_id` | Required FK, explicit FK column name `custom_id` |
| `name:references?` | Nullable FK to the `names` table |
| `name:references?:custom_id` | Nullable FK, explicit FK column name |

Example: `director:references award:references:prize_id` on a `movies` model adds a required `director_id` FK to `directors` and a required `prize_id` FK to `awards`.

## Migration-name inference

For `cargo loco generate migration <Name> ...`, `guess_migration_type` (`loco-gen/src/infer.rs:56`) pattern-matches the **snake_cased** migration name to decide what to scaffold:

| Name pattern | Inferred operation |
|---|---|
| `Create<Table>` | `CreateTable` |
| `Add<ref>RefTo<Table>` | `AddReference` |
| `Add<Columns>To<Table>` | `AddColumns` |
| `Remove<Columns>From<Table>` | `RemoveColumns` |
| `Rename<Old>To<New>On<Table>` | `RenameColumn` (takes no `field:type` arguments — the column keeps its type) |
| `CreateJoinTable<A>And<B>` | `CreateJoinTable` — both sides are singularized into `a_b` (`loco-gen/src/migration.rs:54`), then the template pluralizes that name for the table itself (`templates/migration/join_table.t:4`). So `CreateJoinTableUsersAndGroups` creates the table **`user_groups`**, with `user` and `group` reference columns |
| anything else | `Empty` |

```bash
# renames movies.title to movies.name, with a down() that renames it back
cargo loco generate migration RenameTitleToNameOnMovies
```

Multi-word names work — `RenameFirstNameToGivenNameOnUserProfiles` — because the parser anchors on the last `On` and the first `To` before it rather than counting words.

:::caution
`Empty` is the fallback for a name Loco can't read, and its `up()` is `todo!()` — running `cargo loco db migrate` will panic until you write the body. That's deliberate: a stub that silently succeeded would be recorded as applied and your schema would be permanently out of step. The generator says so when it creates one.
:::

## Inflection conventions (`cruet` vs `heck`)

Documented at `loco-gen/src/infer.rs:1-14`: **`cruet`** is used *only* for pluralization/singularization (`to_plural`/`to_singular` — table names); **`heck`** is used for *all* case conversion (snake_case columns, PascalCase entity/struct names). The two crates disagree on acronym/digit casing (e.g. `i32`→`i_32` under `cruet` vs `i32` under `heck`; `HTTPServer`→`Httpserver` under `cruet` vs `HttpServer` under `heck`), so mixing them corrupts generated identifiers. The one deliberate exception: `guess_migration_type` normalizes the raw migration command name with `cruet`'s snake-casing before splitting it into keyword parts, because the parser is tuned to that specific behavior.

## Related reference pages

- [Feature flags](/docs/reference/feature-flags) — `with-db` and the other Cargo features gating generators.
- Schema/`ColType` migration DSL and query pagination reference pages cover the migration-writer side (`add_column`, `add_reference`, `ColType`) in depth.
