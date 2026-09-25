---
title: Додати базу даних до наявного застосунку
description: "Перетворіть застосунок, згенерований з --db none, на застосунок із базою даних: увімкніть with-db, створіть крейт міграцій та підключіть Hooks, бінарники й конфігурацію."
sidebar:
  order: 6
---

**Мета:** додати базу даних до застосунку, згенерованого **без** неї.

Вибір «без бази даних» у `loco new` — це не дрібне перемикання: воно вимикає можливість `with-db`, що прибирає `AppContext::db`, крейт `migration`, модуль `models` і два обов'язкові методи `Hooks`. Жоден генератор цього не відкочує, тож ця сторінка і є процедурою. Заплануйте пів години.

:::note
Якщо ви лише *починаєте* новий застосунок, оберіть базу даних у запиті майстра. Це набагато дешевше, ніж усе описане на цій сторінці.
:::

## 1. Увімкніть можливість знову

Застосунок, створений з `--db none`, фіксує `loco-rs` зі стандартними можливостями, що вимкнені:

```toml
# Cargo.toml — до
loco-rs = { workspace = true, features = ["cli"] }
```

`with-db` — це стандартна можливість, тож розв'язання полягає в тому, щоб припинити вимикати стандартні можливості. У `[workspace.dependencies]` приберіть `default-features = false`, а потім додайте залежності бази даних, які застосунок із БД має з собою:

```toml
# Cargo.toml — після
[workspace.dependencies]
loco-rs = { version = "1.1" }        # без `default-features = false`

[dependencies]
loco-rs = { workspace = true }
migration = { path = "migration" }
sea-orm = { version = "2.0", features = [
  "sqlx-sqlite",
  "sqlx-postgres",
  "runtime-tokio-rustls",
  "macros",
] }
chrono = { version = "0.4" }
validator = { version = "0.20" }
uuid = { version = "1.6", features = ["v4"] }
```

Також додайте `ts-rs = { version = "12", features = ["chrono-impl", "serde-compat"] }`, якщо хочете типізовані DTO-прив'язки — дивіться [Build a typed React SPA](/uk/docs/how-to/build-a-spa/).

## 2. Створіть крейт `migration`

Це крейт-сусід у `migration/`, на який посилаються через шлях. Два файли:

```toml
# migration/Cargo.toml
[package]
name = "migration"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
name = "migration"
path = "src/lib.rs"

[dependencies]
loco-rs = { workspace = true }

[dependencies.sea-orm-migration]
version = "2.0"
features = ["runtime-tokio-rustls"]
```

```rust
// migration/src/lib.rs
#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // inject-above (do not remove this comment)
        ]
    }
}
```

:::caution
Залиште рядок `// inject-above (do not remove this comment)` точно таким, як він написаний. `cargo loco generate model` реєструє кожну нову міграцію, вставляючи її над цим рядком. Без якоря генератор падає — гучно, і це навмисно: до Loco 1.1 він повідомляв про успіх і мовчки залишав міграцію незареєстрованою.
:::

## 3. Додайте модуль `models`

```
src/models/
├── mod.rs              # pub mod _entities;
└── _entities/
    ├── mod.rs          # pub mod prelude;
    └── prelude.rs      # (поки що порожній)
```

Потім оголосіть його у `src/lib.rs`:

```rust
pub mod models;
pub mod dtos;   // лише якщо ви додали ts-rs у кроці 1
```

`_entities/` — це згенерований код: `cargo loco db entities` переписує його з чинної схеми. Власну логіку моделі розміщуйте у `src/models/<name>.rs` поруч, а ніколи — всередині `_entities/`.

## 4. Підключіть `Hooks` і бінарники

Саме тут виникають помилки компілятора, через які зазвичай і звертаються до цієї сторінки, — і всі вони суто механічні.

У `src/app.rs` функція `boot` отримує параметр типу `Migrator`:

```rust
use migration::Migrator;

async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult> {
    create_app::<Self, Migrator>(mode, environment, config).await
}
```

`with-db` також робить **ще два методи `Hooks` обов'язковими** — у них немає типових реалізацій, тож `impl Hooks for App` не скомпілюється, поки обидва не існують:

```rust
use std::path::Path;

async fn truncate(_ctx: &AppContext) -> Result<()> {
    Ok(())
}

async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
    Ok(())
}
```

Порожні тіла підходять для початку; заповніть їх, коли знадобляться (дивіться [Seed data](/uk/docs/how-to/seed-data/).

Обидва бінарники приймають той самий параметр:

```rust
// src/bin/main.rs та src/bin/tool.rs
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
```

А `tests/mod.rs` отримує свій модуль models, щойно у вас з'являться тести моделей:

```rust
mod models;
```

## 5. Налаштуйте з'єднання

Додайте блок `database:` до конфігурації **кожного** середовища — `config/development.yaml`, `config/test.yaml` та `config/production.yaml`. Production не має типових значень, тож він має читати їх із середовища:

```yaml
# config/development.yaml
database:
  uri: <%= get_env(name="DATABASE_URL", default="sqlite://myapp_development.sqlite?mode=rwc") %>
  enable_logging: false
  connect_timeout: 500
  idle_timeout: 500
  min_connections: 1
  max_connections: 1
  auto_migrate: true
  dangerously_truncate: false
  dangerously_recreate: false
```

```yaml
# config/production.yaml
database:
  uri: <%= get_env(name="DATABASE_URL") %>
  auto_migrate: false
  dangerously_truncate: false
  dangerously_recreate: false
```

Використовуйте `sqlite://…?mode=rwc` для SQLite або `postgres://user:pass@host:5432/dbname` для Postgres. Ніколи не вмикайте `dangerously_truncate` чи `dangerously_recreate` поза development і test.

## 6. Перевірте

```sh
cargo build                       # компілюється обв'язка Hooks/Migrator
cargo loco db status              # з'єднання працює
cargo loco generate model post title:string! content:text
cargo loco db migrate
cargo loco db entities            # потребує: cargo install sea-orm-cli
cargo loco start
```

Якщо `generate model` повідомляє, що не може виконати вставку до `migration/src/lib.rs`, якірний коментар з кроку 2 відсутній або був переформатований.

## Далі

- [Додати модель](/uk/docs/how-to/add-model/) — синтаксис полів і звичайний робочий процес звідси й далі
- [Запит даних](/uk/docs/how-to/query-data/)
- [Кілька баз даних](/uk/docs/how-to/multi-database/) — якщо потрібне більше ніж одне з'єднання
