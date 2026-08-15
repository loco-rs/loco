+++
title = "Generate code with cargo loco generate"
description = "Scaffold models, migrations, controllers, workers, and more with `cargo loco generate <kind>`, using the shared field:type mini-language."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 60
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

Goal: scaffold application code (models, migrations, controllers, workers, mailers, deployment files, ...) from Loco's built-in templates instead of hand-writing boilerplate.

## 1. Know the constraint: debug builds only

`cargo loco generate` (alias `g`) is compiled only in debug builds — `#[cfg(debug_assertions)]` gates the whole subcommand (`src/cli.rs`). It's available whenever you run your app the normal dev way (`cargo run`, `cargo loco start`, `cargo test`), but it is **not present in a `--release` binary**. `model`/`migration`/`scaffold` are additionally gated on the `with-db` feature (on by default).

## 2. Run a generator

```sh
# an empty model (entity + migration + test)
cargo loco generate model posts

# a model with typed fields
cargo loco generate model posts title:string! content:text

# a full CRUD resource: entity + migration + controller + routes + tests
cargo loco generate scaffold posts title:string! user:references

# controller only, no model/migration
cargo loco generate controller posts index show

# non-DB generators
cargo loco generate task cleanup_old_sessions
cargo loco generate worker send_digest
cargo loco generate mailer welcome
cargo loco generate scheduler
cargo loco generate data countries
cargo loco generate deployment docker
```

Every generator writes files relative to your project root and prints what it created (or, for `model`/`migration`/`scaffold`, injects a `mod` line into the relevant `mod.rs`).

## 3. Pick the right kind

| Kind | Needs `with-db` | What you get |
|---|---|---|
| `model` | yes | Sea-ORM entity + model file + migration + a starter test in `tests/models/` |
| `migration` | yes | Standalone migration file (add/remove columns, join tables, or an empty stub — inferred from the name) |
| `scaffold` | yes | Full CRUD: entity, migration, controller, routes, tests — plus typed React hooks/pages when the app has a `frontend/` |
| `controller` | no | Controller + routes + tests, no model |
| `task` | no | One-off/CLI task stub, registered automatically |
| `scheduler` | no | `config/scheduler.yaml` starter |
| `worker` | no | Background worker stub, registered automatically |
| `mailer` | no | Mailer struct + embedded `subject`/`html`/`text` templates |
| `data` | no | Data-loader struct + a static `data/<name>/data.json` |
| `deployment` | no | `docker` or `nginx` deployment files |
| `override` | no | Copies a built-in template locally so you can edit it — see [Override built-in templates](@/docs/how-to/override-templates.md) |

`scaffold` and `controller` are **adaptive** — there's no kind flag to pick. `controller` always generates a JSON API controller; `scaffold` generates a JSON API controller and, when the app has a `frontend/`, also emits typed React hooks and pages for the resource. The old `--html`/`--htmx` server-side views are gone (replaced by the React SPA frontend); `--api` is still accepted as a no-op for back-compat.

This is a summary for orientation only — the exhaustive, verified dictionary of every kind, every flag, and migration-name inference rules is the [Generators & field types reference](@/docs/reference/generators.md); the raw CLI flag shapes are also in the [CLI reference](@/docs/reference/cli.md#2-4-generate-subcommands).

## 4. Use the field-type mini-language

`model`, `migration`, and `scaffold` all take `name:type` pairs after the resource name. The full table of ~50 base types (with their `!`/`^` suffix variants, arities, and Rust types) lives in the [field-type mini-language reference](@/docs/reference/generators.md#field-type-mini-language) — check it before guessing a type name. A few load-bearing facts to keep in mind while typing field lists:

- No suffix = nullable (`Option<T>`); `!` = required; `^` = unique (implies required). Not every type has a `^` form (`bool`, `tstz`, `json` don't).
- **`int` is `i64`/`BIGINT`** in Loco 1.0 (it was `i32` before) — `big_int` is just an alias. Use `small_int`/`small_unsigned` if you need a 16-bit column.
- `name:references` adds a required belongs-to foreign key (`name_id`); `name:references?` makes it nullable; `name:references:custom_id` (optionally with `?`) picks the FK column name explicitly.
- `array` types take the element type as a second colon segment: `tags:array:string`, `scores:array!:int`.

```sh
cargo loco generate model movies long_title:string director:references award:references:prize_id
```

## 5. Apply generated migrations

Generating a `migration` (standalone or via `scaffold`/`model`) only writes the file — it doesn't touch the database. Apply it and regenerate entities:

```sh
cargo loco db migrate && cargo loco db entities
```

## Verify it

```sh
cargo build          # generators need a debug build to even be available
cargo loco generate model posts title:string!
cargo loco db migrate
cargo test
```

A successful generator run prints the list of files it created/modified; `cargo build` (or `cargo check`) then confirms the generated code compiles, and `cargo test` runs the starter test the generator scaffolded for you.
