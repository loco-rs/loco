---
title: Generators & field types
description: Every `cargo loco generate <kind>` component, its exact CLI syntax and output files, plus the complete field-type mini-language used by model/migration/scaffold generators.
sidebar:
  order: 3
---

`cargo loco generate <kind>` (alias `cargo loco g <kind>`) scaffolds application code from templates baked into the `loco-gen` crate. The `generate` subcommand itself is compiled only under `#[cfg(debug_assertions)]` (`src/cli.rs:140`) — it is available in ordinary (dev/debug) builds but is compiled out of `--release` binaries. The kinds that touch the database (`model`, `migration`, `scaffold`) are additionally gated behind the `with-db` Cargo feature (on by default) — see [feature flags](@/docs/reference/feature-flags.md).

This page is the exhaustive dictionary of generator kinds and the field-type mini-language (`name:type`) they all share. It transcribes `loco-gen/src/lib.rs` (the `Component` and `ScaffoldKind` enums), `loco-gen/src/mappings.json` (the field-type table), `loco-gen/src/infer.rs` (naming/inflection conventions), and `src/cli.rs` (the CLI surface), re-verified against `HEAD`.

## Generator kinds

`Component` enum: `loco-gen/src/lib.rs:237`. CLI subcommand enum `ComponentArg`: `src/cli.rs:175` (also gated `#[cfg(debug_assertions)]`).

| Kind | CLI syntax | Feature gate | Notes |
|---|---|---|---|
| **model** | `cargo loco generate model <name> [field:type ...] [--without-tz]` | `with-db` | Creates a Sea-ORM entity + model file + migration + tests. `lib.rs:239`, `cli.rs:197` |
| **migration** | `cargo loco generate migration <name> [field:type ...] [--without-tz]` | `with-db` | Standalone migration file; no model/entity. Name-based operation inference (create/add/remove/join) — see [Migration-name inference](#migration-name-inference). `lib.rs:250`, `cli.rs:250` |
| **scaffold** | `cargo loco generate scaffold <name> [field:type ...] (--api\|--html\|--htmx) [--without-tz]` | `with-db` | Full CRUD: entity, migration, controller, routes, views (for `--html`/`--htmx`), tests. `lib.rs:261`, `cli.rs:268` |
| **controller** | `cargo loco generate controller <name> [action ...] (--api\|--html\|--htmx)` | none | Controller + routes + tests only — no model/migration. `lib.rs:274`, `cli.rs:307` |
| **task** | `cargo loco generate task <name>` | none | One-off/CLI task stub, registered in `src/tasks/mod.rs`. `lib.rs:284` |
| **scheduler** | `cargo loco generate scheduler` | none | Writes `config/scheduler.yaml`. `lib.rs:288` |
| **worker** | `cargo loco generate worker <name>` | none | Background worker stub in `src/workers/`, registered in `src/workers/mod.rs`. `lib.rs:289` |
| **mailer** | `cargo loco generate mailer <name>` | none | Mailer struct in `src/mailers/<name>.rs` + embedded `welcome/{subject,html,text}.t` templates. `lib.rs:293` |
| **data** | `cargo loco generate data <name>` | none | Data-loader struct + a `data/<name>/data.json` static file. `lib.rs:297` |
| **deployment** | `cargo loco generate deployment <docker\|nginx>` | none | `kind` is a **positional** value, not a `--kind` flag (see [Deployment](#deployment)). `lib.rs:301` |
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

# scaffold (model + controller + views + tests), API-only
cargo loco generate scaffold posts title:string! user:references --api
```

After generating a `migration`, apply it and regenerate entities: `cargo loco db migrate && cargo loco db entities`.

### Scaffold / controller kind (`--api` / `--html` / `--htmx`)

`ScaffoldKind` (`loco-gen/src/lib.rs:218`):

```rust
pub enum ScaffoldKind {
    Api,
    Html,
    Htmx,
}
```

`scaffold` and `controller` both require **exactly one** of `--kind <api|html|htmx>`, `--api`, `--html`, or `--htmx` (they're a clap argument group). **There is no default** — omitting all of them is a hard error: `Error: generating this component requires one of --kind, --htmx, --html, or --api to be specified` (scaffold: `src/cli.rs:428`; controller: `src/cli.rs:457`).

### Deployment

`DeploymentKind` in `loco-gen` carries generator data (`loco-gen/src/lib.rs:224`):

```rust
pub enum DeploymentKind {
    Docker { copy_paths: Vec<PathBuf>, is_client_side_rendering: bool },
    Nginx { host: String, port: i32 },
}
```

but the **CLI-facing** enum (`src/cli.rs:555`) is a plain `clap::ValueEnum { Docker, Nginx }` taken as a positional argument — `copy_paths`/`is_client_side_rendering`/`host`/`port` are derived from the app's own `config/*.yaml` and filesystem at generation time (`src/cli.rs:562-596`), not passed on the command line:

```bash
cargo loco generate deployment docker   # writes Dockerfile, .dockerignore
cargo loco generate deployment nginx    # writes nginx/default.conf
```

### Override

Copies a built-in `.t` template (or a whole folder) into the local `.loco-templates/` directory (`DEFAULT_LOCAL_TEMPLATE`, `loco-gen/src/template.rs:8`) so subsequent generation runs use your copy instead of the built-in one. Delete the local copy to revert to the built-in template.

```bash
# list all overridable templates
cargo loco generate override

# override one file
cargo loco generate override scaffold/api/controller.t

# override every template under a folder
cargo loco generate override scaffold/htmx

# preview what --info would show for a folder, without copying
cargo loco generate override scaffold/htmx --info

# override everything
cargo loco generate override .
```

## Field-type mini-language

Every `field:type` argument to `model`/`migration`/`scaffold` is resolved through the type table baked into `loco-gen/src/mappings.json`. Transcribed in full below (re-verified against `HEAD`).

**Suffix convention:** no suffix = nullable `Option<T>`; **`!`** = required (non-null); **`^`** = unique (implies non-null). Not every base type has all three variants — `bool`, `tstz`, and `json` have no `^` (unique) form.

**1.0 change:** `int` now maps to **`i64` / `BIGINT`** (`big_integer`), matching the framework's i64 primary keys. Pre-1.0, `int` was `i32`. `unsigned` is an alias of `big_unsigned` (also i64). Use `small_int`/`small_unsigned` if you specifically need 16-bit columns.

| `type` (suffix variants) | Rust type | `ColType` variant | Arity |
|---|---|---|---|
| `uuid` / `uuid!` / `uuid^` | `Option<Uuid>` / `Uuid` / `Uuid` | `UuidNull` / `Uuid` / `UuidUniq` | — |
| `string` / `string!` / `string^` | `Option<String>` / `String` / `String` | `StringNull` / `String` / `StringUniq` | — |
| `text` / `text!` / `text^` | `Option<String>` / `String` / `String` | `TextNull` / `Text` / `TextUniq` | — |
| `small_int` / `!` / `^` | `Option<i16>` / `i16` / `i16` | `SmallIntegerNull` / `SmallInteger` / `SmallIntegerUniq` | — |
| `small_unsigned` / `!` / `^` | `Option<i16>` / `i16` / `i16` | `SmallUnsignedNull` / `SmallUnsigned` / `SmallUnsignedUniq` | — |
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
| `date_time` / `!` / `^` | `Option<DateTime>` / `DateTime` / `DateTime` | `DateTimeNull` / `DateTime` / `DateTimeUniq` | — |
| `json` / `!` (no `^`) | `Option<serde_json::Value>` / `serde_json::Value` | `JsonNull` / `Json` | — |
| `jsonb` / `!` / `^` | `Option<serde_json::Value>` / `serde_json::Value` / `serde_json::Value` | `JsonBinaryNull` / `JsonBinary` / `JsonBinaryUniq` | — |
| `blob` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `BlobNull` / `Blob` / `BlobUniq` | — |
| `money` / `!` / `^` | `Option<Decimal>` / `Decimal` / `Decimal` | `MoneyNull` / `Money` / `MoneyUniq` | — |
| `binary_len` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `BinaryLenNull` / `BinaryLen` / `BinaryLenUniq` | **1** (length) |
| `var_binary` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `VarBinaryNull` / `VarBinary` / `VarBinaryUniq` | **1** (length) |
| `array` / `!` / `^` | `Option<Vec<T>>` (see below) | `array_null` / `array` / `array_uniq` (generator emits `ColType::array(ArrayColType::…)` etc.) | **1** (element type) |

`decimal`, `money`, and `decimal_len` all resolve to the same Rust type (`rust_decimal::Decimal`); the `ColType` distinguishes the SQL representation.

### Arrays

`array`/`array!`/`array^` take one parameter — the element type — written as a second colon segment: `tags:array:string`, `scores:array!:int`. Valid element types (per `mappings.json`'s `array` entry) are `string`, `int`, `big_int`, `float`, `double`, `bool`, generating `Option<Vec<T>>` where `T` is:

| element | Rust `T` |
|---|---|
| `string` | `String` |
| `int` | **`i32`** |
| `big_int` | `i64` |
| `float` | `f32` |
| `double` | `f64` |
| `bool` | `bool` |

⚠ **Inconsistency worth knowing:** the scalar `int` type is `i64` in 1.0, but `array:int`'s element type is still `i32` (`loco-gen/src/mappings.json`, the `array` entry's `rust.int` key) — the i64 migration did not touch array element types. Confirmed by `loco-gen/src/model.rs:171-182` (`test_get_columns_with_array_types`), which asserts `array:string` → `array_null(ArrayColType::String)`.

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
| `CreateJoinTable<A>And<B>` | `CreateJoinTable` (join table `a_b`, both sides singularized) |
| anything else | `Empty` (blank migration stub) |

## Inflection conventions (`cruet` vs `heck`)

Documented at `loco-gen/src/infer.rs:1-14`: **`cruet`** is used *only* for pluralization/singularization (`to_plural`/`to_singular` — table names); **`heck`** is used for *all* case conversion (snake_case columns, PascalCase entity/struct names). The two crates disagree on acronym/digit casing (e.g. `i32`→`i_32` under `cruet` vs `i32` under `heck`; `HTTPServer`→`Httpserver` under `cruet` vs `HttpServer` under `heck`), so mixing them corrupts generated identifiers. The one deliberate exception: `guess_migration_type` normalizes the raw migration command name with `cruet`'s snake-casing before splitting it into keyword parts, because the parser is tuned to that specific behavior.

## Related reference pages

- [Feature flags](@/docs/reference/feature-flags.md) — `with-db` and the other Cargo features gating generators.
- Schema/`ColType` migration DSL and query pagination reference pages cover the migration-writer side (`add_column`, `add_reference`, `ColType`) in depth.
