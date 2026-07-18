---
title: Snapshot tests with fixtures and redactions
description: Use insta snapshots for models and responses, redact dynamic fields with cleanup_user_model/cleanup_email, and assert on rendered HTML with select().
sidebar:
  order: 52
---

Goal: snapshot a model, request response, or rendered HTML with [insta](https://crates.io/crates/insta), without the snapshot flapping every run because of a fresh UUID, timestamp, or password hash.

## 1. Enable insta

```toml
[dev-dependencies]
loco-rs = { version = "*", features = ["testing"] }
insta = { version = "*", features = ["redactions"] }
```

Loco does **not** re-export `insta` — it's a regular dev-dependency of your app. What Loco provides is the *filter tables* (`src/testing/redaction.rs`) you feed into insta's `filters` setting.

## 2. Snapshot a value with `assert_debug_snapshot!`

```rust
use insta::assert_debug_snapshot;
use loco_rs::testing::prelude::*;

let boot = boot_test::<App>().await.unwrap();
let user = users::Model::find_by_email(&boot.app_context.db, "user1@example.com").await;

assert_debug_snapshot!(user);
```

The first run writes a `.snap` file under a `snapshots/` folder next to your test; review and accept it with `cargo insta review` (or by hand), then future runs diff against it.

## 3. Redact dynamic fields before snapshotting

A freshly created user has a random UUID `pid`, an incrementing `id`, a bcrypt `password` hash, and `created_at`/`updated_at` timestamps — all of which change every run and would break the snapshot. Wrap the assertion in `insta::with_settings!` with one of Loco's cleanup filter sets:

```rust
use insta::{assert_debug_snapshot, with_settings};
use loco_rs::testing::prelude::*;

let res = Model::create_with_password(&boot.app_context.db, &params).await;

with_settings!({
    filters => cleanup_user_model()
}, {
    assert_debug_snapshot!(res);
});
```

| Filter fn | What it redacts | Placeholder |
|---|---|---|
| `cleanup_user_model()` | UUIDs/PIDs, bcrypt `password: "..."` hashes, JWT-shaped tokens, ISO-8601 timestamps (with and without timezone), `id: <number>` | `PID`, `"PASSWORD"`, `TOKEN`, `DATE`, `id: ID` |
| `cleanup_email()` | Mailer message identifiers, RFC-2822 dates, UUID-shaped random ids | `IDENTIFIER`, `DATE`, `RANDOM_ID` |

Both combine a base table with the shared date filter (`get_cleanup_date()`); `cleanup_user_model()` additionally folds in `get_cleanup_model()` (the `id: N` → `id: ID` rule). If you need a custom combination, the individual tables (`get_cleanup_user_model()`, `get_cleanup_date()`, `get_cleanup_model()`, `get_cleanup_mail()`) are public too — build your own `Vec<(&str, &str)>` filter list from them.

Use `cleanup_email()` the same way when snapshotting mailer deliveries (`ctx.mailer.unwrap().deliveries()`):

```rust
with_settings!({
    filters => cleanup_email()
}, {
    assert_debug_snapshot!(ctx.mailer.unwrap().deliveries());
});
```

## 4. Give each test file its own snapshot namespace

Snapshot filenames are derived from the test function name — across test files that's usually enough, but if two files both have a test with the same name, or you simply want per-file namespacing, define a small local macro (this is *not* a Loco API — it's plain insta, one line of glue code repeated per test file):

```rust
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("users"); // suffixes every snapshot in this file
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn can_find_by_pid() {
    configure_insta!();
    // ...
}
```

`cargo loco generate model`/`scaffold` already scaffold this macro (without the suffix line) into the generated `tests/models/<name>.rs` — see [Using generators](/docs/how-to/use-generators).

## HTML assertions with `select()`

For server-rendered (HTML/HTMX) views, don't snapshot or string-match raw markup — parse it with Loco's `scraper`-backed selector helpers (`src/testing/selector.rs`) instead. All of them panic with a descriptive message on failure, so they read like normal assertions:

```rust
use loco_rs::testing::prelude::*;

let html = response.text();

assert_css_exists(&html, ".flash-message");
assert_css_not_exists(&html, ".error");
assert_css_eq(&html, "h1.title", "Welcome to Loco");
assert_link(&html, "a.home", "/");
assert_attribute_exists(&html, "form", "action");
assert_attribute_eq(&html, "input[name=email]", "type", "email");
assert_count(&html, "ul#posts li", 3);
assert_css_eq_list(&html, "ul#posts li", &["Post 1", "Post 2", "Post 3"]);
```

`select(html, selector) -> Vec<String>` returns the outer HTML of every match, for cases where you want to snapshot a fragment instead of asserting on it directly (combine it with `assert_debug_snapshot!` and the redaction filters above if the fragment contains dynamic data):

```rust
let items = select(&html, ".item");
assert_debug_snapshot!(items);
```

## Verify it

```sh
cargo test
```

To review/update snapshots interactively after intentional output changes, install and run [`cargo-insta`](https://crates.io/crates/cargo-insta):

```sh
cargo install cargo-insta
cargo insta review
```
