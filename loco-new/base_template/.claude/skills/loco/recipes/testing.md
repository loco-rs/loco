# Recipe: testing

Loco ships the harness. `use loco_rs::testing::prelude::*;` gives you app boot,
a database, and an HTTP client — do not stand up a server or a connection
yourself.

## Request tests

`myapp` below is a placeholder for **your crate's name** — the `name` under
`[package]` in `Cargo.toml`. Getting this wrong produces
`cannot find module or crate`.

```rust
use loco_rs::testing::prelude::*;
use myapp::app::App;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn can_register() {
    request::<App, _, _>(|request, ctx| async move {
        let response = request
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "name": "loco",
                "email": "test@loco.com",
                "password": "12341234"
            }))
            .await;

        assert_eq!(response.status_code(), 200);

        // `ctx` is the real AppContext — assert on persisted state too.
        let saved = users::Model::find_by_email(&ctx.db, "test@loco.com").await;
        assert!(saved.is_ok());
    })
    .await;
}
```

`request::<App, _, _>(...)` boots the app, hands you an HTTP client and the
`AppContext`, and tears down afterwards.

**`#[serial]` is not optional** for tests that touch the database. Without it,
concurrent tests share one database and interfere — a class of flake that looks
like a real bug.

## Asserting that mail was sent

With `mailer: { stub: true }` in `config/test.yaml`, nothing leaves the process
and every message is recorded. Read them off the context — there is no mock to
install and no SMTP server to stand up:

```rust
#[tokio::test]
#[serial]
async fn deactivating_emails_the_user() {
    request::<App, _, _>(|request, ctx| async move {
        // ... do the thing that should send mail ...

        let deliveries = ctx.mailer.unwrap().deliveries();
        assert_eq!(deliveries.count, 1);
        assert!(deliveries.messages[0].contains("your account is closed"));
    })
    .await;
}
```

`deliveries()` returns `Deliveries { count: usize, messages: Vec<String> }`,
where each message is the full rendered mail including headers. It is available
under the `testing` feature, which the generated app already enables for tests.

`ctx.mailer` is an `Option` — it is `None` when the config has no `mailer:`
block, so a bare `.unwrap()` failing here means the block is missing from
`config/test.yaml`, not that mail was not sent.

## Model tests

```rust
#[tokio::test]
#[serial]
async fn can_create_user() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let user = users::Model::create_with_password(&boot.app_context.db, &params).await;
    assert!(user.is_ok());
}
```

## Test configuration

`config/test.yaml` is what makes tests deterministic:

```yaml
workers:
  mode: ForegroundBlocking     # jobs run inline — no race with assertions
mailer:
  stub: true                   # capture mail instead of sending it
database:
  dangerously_truncate: true   # clean slate per run
```

`ForegroundBlocking` is the important one. With a real queue, `perform_later`
returns before the job runs and your assertion races it.

## Fixtures and seeds

Put YAML fixtures under `src/fixtures/` and load them:

```rust
seed::<App>(&boot.app_context).await.unwrap();
```

## Snapshots

The repo convention is `insta`. Redact anything non-deterministic — ids,
timestamps, tokens — or the snapshot fails on every run:

```rust
use insta::{assert_debug_snapshot, with_settings};

with_settings!({filters => cleanup_user_model()}, {
    assert_debug_snapshot!(user);
});
```

`loco_rs::testing::prelude` ships redaction helpers (`cleanup_user_model`,
`cleanup_date`, `cleanup_email`) — use them rather than hand-writing regexes.

Review the `.snap` diff before accepting a regenerated snapshot, and delete any
stray `*.snap.new`.

## Running

```sh
cargo test                       # everything
cargo test --all-features
cargo test can_register          # one test by name
```

## Before calling work done

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

A change is not finished until all three are clean.
