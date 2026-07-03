+++
title = "Add a model"
description = "Generate a model with a migration, add fields to it, and run the migration to get working entities."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

**Goal:** add a new database-backed model to a Loco app — a migration, a Sea-ORM entity, and your own model file to extend it — using the model generator.

This assumes a working Loco app with the `with-db` feature enabled (the default). For the full field-type mini-language and every generator kind, see [Generators & field types](@/docs/reference/generators.md). For the migration DSL used under the hood, see [Schema & ColType DSL](@/docs/reference/schema-dsl.md).

## 1. Generate the model

Run the model generator with a name and a list of `field:type` pairs:

```sh
$ cargo loco generate model posts title:string! content:text user:references
```

This does three things in one step:

1. Writes a migration under `migration/src/` that creates a `posts` table.
2. Applies the migration against your development database.
3. Regenerates Sea-ORM entities into `src/models/_entities/`, and scaffolds `src/models/posts.rs` for your own model code.

You end up with:

```
src/
  models/
    _entities/
      posts.rs   <-- generated entity (Entity, Model, ActiveModel, Column, Relation)
    posts.rs     <-- your extension point
migration/
  src/
    m20240101_000002_posts.rs
```

Set the `SKIP_MIGRATION` environment variable if you want the generator to only write the migration file, without applying it or regenerating entities — useful when scripting several `generate model` calls back to back before running `db migrate` once at the end.

## 2. Read the field syntax

Each `field:type` pair follows a small suffix convention:

- no suffix → nullable column (`Option<T>`)
- `!` → required column (`NOT NULL`)
- `^` → unique column (implies `NOT NULL`)

So `title:string!` is a required `String`, and `content:text` is a nullable `Option<String>`.

`user:references` is special: it doesn't name a column type, it declares a belongs-to foreign key. It adds a required `user_id` column referencing the `users` table (`user:references?` makes it nullable; `user:references:author_id` picks a custom column name). See [Generators & field types § References](@/docs/reference/generators.md#references-belongs-to-foreign-keys) for the full syntax.

<div class="infobox">
1.0 change: the generator's <code>int</code>/<code>int!</code>/<code>int^</code> field type now maps to <b>i64 / BIGINT</b> (<code>big_integer</code>), not <code>i32</code> as in earlier Loco versions — matching the framework's i64 auto-increment primary keys. Use <code>small_int</code> if you specifically need a 16-bit column.
</div>

## 3. Add more fields to an existing model

To add columns to a table you already created, generate a plain migration instead of a new model — name it `Add<Columns>To<Table>` so Loco infers an "add columns" migration:

```sh
$ cargo loco generate migration AddViewsToPosts views:int
```

Apply it and regenerate entities:

```sh
$ cargo loco db migrate
$ cargo loco db entities
```

Removing columns follows the mirror-image naming convention, `Remove<Columns>From<Table>`:

```sh
$ cargo loco generate migration RemoveViewsFromPosts views:int
```

## 4. Generate without timestamps (optional)

By default every table generated through the DSL gets `created_at`/`updated_at` columns. To opt out, pass `--without-tz` to `model`, `migration`, or `scaffold`:

```sh
$ cargo loco generate model posts title:string! content:text --without-tz
```

<div class="infobox">
The flag is <code>--without-tz</code>, not <code>--without-timestamps</code> — an older spelling that no longer works.
</div>

## 5. Verify

Confirm the migration applied and entities exist:

```sh
$ cargo loco db status
$ ls src/models/_entities/
```

Then write against the model directly — e.g. in a `cargo loco playground` script or a test:

```rust
use migration::Migrator;
use loco_rs::testing::prelude::*;
use myapp::models::_entities::posts;

let boot = boot_test::<App, Migrator>().await?;
let post = posts::ActiveModel {
    title: sea_orm::ActiveValue::set("hello".to_string()),
    user_id: sea_orm::ActiveValue::set(1),
    ..Default::default()
}
.insert(&boot.app_context.db)
.await?;

assert_eq!(post.title, "hello");
```

**Result:** a `posts` table exists in your database, `posts::Entity`/`Model`/`ActiveModel` compile, and `src/models/posts.rs` is where you add custom methods (e.g. `Model::find_by_title`) the same way `examples/demo/src/models/users.rs` extends the generated `users` entity.

## Next

- [Query data](@/docs/how-to/query-data.md) with the `ConditionBuilder` DSL.
- [Add relationships and validation](@/docs/the-app/models.md) beyond a single table.
