---
title: Seed data
description: "Load fixture data into a fresh database, wire it into Hooks::seed, and dump/import table contents with cargo loco db seed."
sidebar:
  order: 4
---

**Goal:** populate a freshly-migrated database with known rows from YAML fixture files — for local development, tests, and reproducing an environment's data elsewhere.

This assumes a working model (see [Add a model](@/docs/how-to/add-model.md)).

## 1. Write a fixture file

Fixtures live under `src/fixtures/`, one YAML file per table, containing a plain list of records:

```
src/
  fixtures/
    users.yaml
```

```yaml
# examples/demo/src/fixtures/users.yaml
---
- id: 1
  pid: 11111111-1111-1111-1111-111111111111
  email: user1@example.com
  password: "$argon2id$v=19$m=19456,t=2,p=1$ETQBx4rTgNAZhSaeYZKOZg$eYTdH26CRT6nUJtacLDEboP0li6xUwUF/q5nSlQ8uuc"
  api_key: lo-95ec80d7-cb60-4b70-9b4b-9ef74cb88758
  name: user1
  created_at: "2023-11-12T12:34:56.789Z"
  updated_at: "2023-11-12T12:34:56.789Z"
```

Include every `NOT NULL` column your migration defined; nullable columns can be omitted.

## 2. Wire the fixture into `Hooks::seed`

Add a call to `db::seed::<ActiveModel>` in your app's `Hooks::seed` implementation, one line per fixture file:

```rust
use std::path::Path;
use loco_rs::{app::{AppContext, Hooks}, db, Result};

impl Hooks for App {
    // ...
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        Ok(())
    }
}
```

`db::seed` reads the YAML into `Vec<serde_json::Value>`, converts each row through `A::from_json`, inserts them with `insert_many`, and resets the table's auto-increment sequence afterward so subsequently-created rows don't collide with the seeded IDs.

## 3. Run the seed command

```sh
$ cargo loco db seed
```

By default this reads from `src/fixtures` and inserts into whatever environment you're targeting (`-e`/`--environment`, default `development`). Common flags:

```sh
# clear all data before seeding — useful for a repeatable dev/test reset
$ cargo loco db seed --reset

# seed from a different folder (e.g. environment-specific fixtures)
$ cargo loco db seed --from src/fixtures/staging

# target a specific environment
$ cargo loco db seed -e test
```

## 4. Dump existing data back to fixtures

The same command can go the other direction: export live table contents to YAML files, e.g. to capture a snapshot of production-like data for local fixtures.

```sh
# dump every table to --from's folder (default: src/fixtures)
$ cargo loco db seed --dump

# dump only specific tables
$ cargo loco db seed --dump-tables users,posts
```

`--dump`/`--dump-tables` and seeding are mutually exclusive for a single invocation — passing either one dumps instead of seeding.

## 5. Use seeding in tests

With the `testing` feature enabled, `boot_test` + `seed::<App>` gives each test a freshly-seeded database:

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_find_seeded_user() {
    let boot = boot_test::<App, Migrator>().await?;
    seed::<App>(&boot.app_context).await?;

    let user = Model::find_by_email(&boot.app_context.db, "user1@example.com").await;
    assert!(user.is_ok());
}
```

## Result

`cargo loco db seed` (optionally with `--reset`) leaves your database populated with the exact rows in `src/fixtures/*.yaml`, and `cargo loco db seed --dump` lets you regenerate those same fixture files from a live database whenever your schema or sample data changes.
