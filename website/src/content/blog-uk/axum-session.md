---
title: Створюємо Rust-застосунок з Axum Session
description: Додайте сесії до свого застосунку за допомогою Axum Sessions. Налаштуйте провайдер сесій і підключіть Axum Session та Loco через прості app-хуки.
pubDate: 2023-12-19
updatedDate: 2023-12-19
authors:
  - team-loco
---

Щоб побудувати Rust-застосунок з [Axum session](https://crates.io/crates/axum_session), перший крок — обрати свій сервер. У цьому випадку ми використаємо [loco](https://loco.rs) :)

Почніть зі створення нового проєкту та вибору шаблону `SaaS app`:

```sh
$ cargo install loco
$ loco new
✔ ❯ App name? · myapp
? ❯ What would you like to build? ›
  lightweight-service (minimal, only controllers and views)
  Rest API (with DB and user auth)
❯ SaaS app (with DB and user auth)
```

## Створення сесії лише з in-memory сховищем

Спершу додайте крейт Axum session до Cargo.toml:

```toml
axum_session = {version = "0.10.1", default-features = false}
```

Потім додайте шар Axum session до свого роутера. Відкрийте app.rs і додайте наступний хук:

```rust
pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    // Інші хуки...
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
    // Інші хуки...
}

```

Тепер можна створити контролер, який використовує Axum session. Скористайтеся командою `cargo loco generate controller`:

```sh
❯ cargo loco generate controller mysession --api
    Finished dev [unoptimized + debuginfo] target(s) in 0.36s
     Running `target/debug/axum-session-cli generate controller mysession`
added: "src/controllers/mysession.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/mysession.rs"
injected: "tests/requests/mod.rs"
```

Відкрийте файл `src/controllers/mysession.rs`, створений генератором контролерів, і замініть його вміст наступним кодом:

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

Тепер ви можете викликати ендпоінт `http://127.0.0.1:5150/mysession`, щоб побачити сесію.

## Створення сесії з шифруванням у БД

Щоб додати шифрування сесій у базі даних, включіть до Cargo.toml крейт Axum session разом із PostgreSQL через SQLx:

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

Створіть файл `session.rs` з наступним вмістом:
Функція `connect_to_database` приймає конфігурацію `Database` і повертає екземпляр PgPool, який очікує axum session.

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

Додайте шар Axum session до свого роутера у `app.rs`:

```rust
use session; // Це файл session.rs
pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    // Інші хуки...
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
    // Інші хуки...
}

```

Створіть контролер так само, як і раніше, за допомогою `cargo loco generate controller`

```sh
❯ cargo loco generate controller mysession --api
    Finished dev [unoptimized + debuginfo] target(s) in 0.36s
     Running `target/debug/axum-session-cli generate controller mysession`
added: "src/controllers/mysession.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/mysession.rs"
injected: "tests/requests/mod.rs"
```

і замініть вміст `src/controllers/mysession.rs` на наведений код.

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

Тепер, коли ви викличете ендпоінт `http://127.0.0.1:5150/mysession`, ви побачите сесію.
