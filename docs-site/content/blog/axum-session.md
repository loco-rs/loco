+++
title = "Building a Rust App with Axum Session"
description = "Building a Rust App with Axum Session"
date = 2023-12-19T09:19:42+00:00
updated = 2023-12-19T09:19:42+00:00
draft = false
template = "blog/page.html"

[taxonomies]
authors = ["Team Loco"]

+++

To build a Rust app with [Axum session](https://crates.io/crates/axum_session), the first step is to choose your server. In this case, we'll use [loco](https://loco.rs) :)

Start by creating a new project and selecting the `React Frontend` template:

```sh
$ cargo install loco-cli
$ loco new
✔ ❯ App name? · myapp
? ❯ What would you like to build? ›
  lightweight-service (minimal, only controllers and views)
  Rest API (with DB and user auth)
❯ React Frontend (with DB and user auth)
```

## Creating Session Memory Store Only

First, add the Axum session crate to Cargo.toml:

```toml
axum_session = {version = "0.10.1", default-features = false}
```

Then, add an Axum session layer to your router. Open app.rs and add the following hook:

```rust
pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    // Other hooks...
    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let session_config =
            axum_session::SessionConfig::default().with_table_name("sessions_table");

        let session_store =
            axum_session::SessionStore::<axum_session::SessionNullPool>::new(None, session_config)
                .await
                .unwrap();

        let router = router.layer(axum_session::SessionLayer::new(session_store));
        Ok(router)
    }
    // Other hooks...
}

```

Now, you can create your controller that uses Axum session. Use the `cargo loco generate controller` command:

```sh
❯ cargo loco generate controller mysession
    Finished dev [unoptimized + debuginfo] target(s) in 0.36s
     Running `target/debug/axum-session-cli generate controller mysession`
added: "src/controllers/mysession.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/mysession.rs"
injected: "tests/requests/mod.rs"
```

Open the `src/controllers/mysession.rs` file created by the controller generator and replace its content with the following code:

```rust
#![allow(clippy::unused_async)]
use axum_session::{Session, SessionNullPool};
use loco_rs::prelude::*;

pub async fn get_session(session: Session<SessionNullPool>) -> Result<()> {
    println!("{:#?}", session);
    format::empty()
}

pub fn routes() -> Routes {
    Routes::new().prefix("mysession").add("/", get(get_session))
}
```

Now, you can call the `http://127.0.0.1:3000/api/mysession` endpoint to see the session.

## Creating Session With DB Encryption

To add session DB encryption, include the Axum session crate and PostgreSQL with SQLx in Cargo.toml:

```toml
axum_session = {version = "0.10.1"}
sqlx = { version = "0.7.2", features = [
  "macros",
  "postgres",
  "_unstable-all-types",
  "tls-rustls",
  "runtime-tokio",
] }

```

Create a `session.rs` file with the following content:
The `connect_to_database` getting an `Database` configuration and returns a PgPool instance that axum session expected.

```rust
use sqlx::postgres::PgPool;
use loco_rs::{
    config::Database,
    errors::Error,
    Result,
};

async fn connect_to_database(config: &Database) -> Result<PgPool> {
    PgPool::connect(&config.uri)
        .await
        .map_err(|e| Error::Any(e.into()))
}

```

Add the Axum session layer to your router in `app.rs`:

```rust
use session; // This is the session.rs file
pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    // Other hooks...
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let conn = session.connect_to_database(&ctx.config.database).await?;
        let session_config = axum_session::SessionConfig::default()
            .with_table_name("sessions_table")
            .with_key(axum_session::Key::generate())
            .with_database_key(axum_session::Key::generate())
            .with_security_mode(axum_session::SecurityMode::PerSession);

        let session_store = axum_session::SessionStore::<axum_session::SessionPgPool>::new(
            Some(conn.clone().into()),
            session_config,
        )
        .await
        .unwrap();

        let router = router.layer(axum_session::SessionLayer::new(session_store));
        Ok(router)
    }
    // Other hooks...
}

```

Create the controller as before using `cargo loco generate controller`

```sh
❯ cargo loco generate controller mysession
    Finished dev [unoptimized + debuginfo] target(s) in 0.36s
     Running `target/debug/axum-session-cli generate controller mysession`
added: "src/controllers/mysession.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/mysession.rs"
injected: "tests/requests/mod.rs"
```

and replace the content of `src/controllers/mysession.rs` with the provided code.

```rust
#![allow(clippy::unused_async)]
use axum_session::{Session, SessionPgPool};
use loco_rs::prelude::*;

pub async fn get_session(session: Session<SessionPgPool>) -> Result<()> {
    println!("{:#?}", session);
    format::empty()
}

pub fn routes() -> Routes {
    Routes::new().prefix("mysession").add("/", get(get_session))
}

```

Now, calling the `http://127.0.0.1:3000/api/mysession` endpoint will display the session.
