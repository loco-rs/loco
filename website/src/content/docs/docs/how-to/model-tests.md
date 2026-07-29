---
title: Write DB model tests
description: Boot a database-backed test app with boot_test, seed fixtures, and get automatic per-test DB cleanup via BootResultWrapper's Drop.
sidebar:
  order: 51
---

Goal: exercise your Sea-ORM models against a real database in a test, with fixture data loaded and the database cleaned up afterwards — with no manual teardown code.

This page covers the `with-db` half of the `testing` feature (`db.rs`): `boot_test`, `seed`, and the two DB-lifecycle strategies Loco supports. For HTTP-level tests, see [request tests](/docs/how-to/request-tests); for insta snapshots/redactions, see [Fixtures & snapshots](/docs/how-to/fixtures-snapshots).

## 1. Enable the features

```toml
[dev-dependencies]
loco-rs = { version = "*", features = ["testing"] }
serial_test = "*"
```

`with-db` (on by default) must also be enabled on your `loco-rs` dependency for `db.rs`'s helpers (`seed`, `boot_test_with_create_db`, `TestSupport`, ...) to be compiled in.

## 2. Pick a DB-lifecycle strategy

Loco supports two ways to run DB tests, and both are legitimate — pick based on whether your test suite shares one test database or spins up a throwaway one per test.

### A. Shared test DB + truncate-before-boot (what `loco new` generates)

Point `config/test.yaml`'s `database.uri` at one test database (SQLite file or Postgres DB) and set `dangerously_truncate: true`. Every `boot_test::<App>()` call then truncates the configured tables before the test body runs, so each test starts from a clean slate — as long as tests run one at a time:

```yaml
# config/test.yaml
database:
  uri: "sqlite://demo_test.sqlite?mode=rwc"
  dangerously_truncate: true
```

```rust
use demo::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial] // required: tests share one DB, so they must not interleave
async fn can_find_by_pid() {
    let boot = boot_test::<App>().await.expect("failed to boot test app");
    seed::<App>(&boot.app_context).await.expect("failed to seed");

    let existing = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111").await;
    assert!(existing.is_ok());
}
```

> ⚠️ `dangerously_truncate` clears data on every `boot_test` call — never point it at a database you care about (never production). Control exactly which tables get truncated via the `truncate` hook on your `Hooks` impl (`async fn truncate(ctx: &AppContext) -> Result<()> { truncate_table(&ctx.db, users::Entity).await }`).

### B. Fresh, unique database per test (no `#[serial]` needed for DB isolation)

`boot_test_with_create_db::<App>()` creates a brand-new, uniquely-named database before booting, and returns a `BootResultWrapper` instead of a plain `BootResult`:

```rust
use demo::app::App;
use loco_rs::testing::prelude::*;

#[tokio::test]
async fn can_register_in_isolated_db() {
    let boot = boot_test_with_create_db::<App>()
        .await
        .expect("failed to boot test app with a fresh db");

    // BootResultWrapper derefs to BootResult, so `boot.app_context` works as usual
    seed::<App>(&boot.app_context).await.expect("failed to seed");
    // ... exercise boot.app_context.db ...

} // <- `boot` drops here: the throwaway database is cleaned up automatically
```

How the unique DB is provisioned depends on the scheme of `config.database.uri` (dispatched in `init_test_db_creation`):

| URI scheme | Backing strategy |
|---|---|
| `postgres://` | Creates a new database named `_loco_test_{10-char-random}_{unix-timestamp}` on the same Postgres server. |
| `sqlite://` | Backs the DB with a `tree-fs` temporary file (`test.sqlite` in a fresh temp dir). |
| anything else | No-op passthrough (`Any`) — used as-is, no isolation. |

### `BootResultWrapper` auto-cleans on `Drop`

`BootResultWrapper` (only present with `with-db`) wraps a `BootResult` plus the `Box<dyn TestSupport>` that created the throwaway DB:

- It `Deref`s to `BootResult`, so `boot.app_context`, `boot.router`, etc. all work exactly like the plain `boot_test` return value.
- Its `Drop` implementation calls `test_db.cleanup_db()` — for Postgres this `DROP DATABASE`s the throwaway DB on a spawned blocking task; for SQLite it removes the temp directory. This runs automatically at the end of the test function's scope, with no explicit teardown call needed.

> **Caveat:** if the test process is killed mid-run (e.g. `Ctrl+C`), `Drop` never fires and the throwaway database/schema is left behind — you'll need to remove it manually (`DROP DATABASE _loco_test_...` for Postgres, or delete the leftover temp dir for SQLite).

## 3. Seed fixture data

`seed::<App>(&ctx)` loads fixtures from the hardcoded `src/fixtures` folder by delegating to your `Hooks::seed` implementation:

```rust
seed::<App>(&boot.app_context).await.expect("failed to seed");
```

This is the same fixture format used by `cargo loco db seed` — see the [`db seed` CLI reference](/docs/reference/cli#2-2-db-subcommands) for the on-disk layout and the `--from <DIR>` flag if you keep fixtures elsewhere.

## 4. Generated model tests already do this for you

`cargo loco generate model <name> ...` (and `scaffold`) scaffold a starter test in `tests/models/<name>.rs` that already follows pattern A above — `boot_test::<App>()`, `seed::<App>()`, a local `configure_insta!()` macro for snapshot naming, and a commented-out `assert_debug_snapshot!` call for you to fill in. See [Using generators](/docs/how-to/use-generators).

## Verify it

```sh
cargo test
```

For pattern A, run the whole suite (or at least the model tests) together so `#[serial]` can do its job — running a single test in isolation (`cargo test can_find_by_pid`) also works, but mixing serial and non-serial DB tests in the same binary without `#[serial]` on all of them will produce flaky failures from concurrent truncation.
