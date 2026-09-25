---
title: Підключення другої бази даних
description: Приєднайте одне або кілька додаткових з'єднань із базами даних за допомогою вбудованого ініціалізатора multi_db.
sidebar:
  order: 5
---

**Мета:** виконувати запити до другої (чи третьої+) бази даних з контролера, окрім основного з'єднання `ctx.db` застосунку — наприклад, до репліки для читання, застарілої бази або баз окремих орендарів.

Loco постачається з готовим [ініціалізатором](/uk/docs/how-to/add-middleware/) для цього: `MultiDbInitializer`, іменована мапою з'єднань. Він живе у `loco_rs::initializers::multi_db` і доступний за можливістю `with-db`. Кожен запис конфігурації приймає ті самі ключі, що й основний блок `database:` — повний перелік дивіться у [Configuration § database](/uk/docs/reference/configuration/) (`uri`, `enable_logging`, `min_connections`, `max_connections`, `connect_timeout`, `idle_timeout`, `acquire_timeout`, `auto_migrate`, `dangerously_truncate`, `dangerously_recreate`, `run_on_start`).

## Налаштуйте кожну іменовану базу даних

Додайте один або кілька записів у мапу `multi_db`, вкладену під ключем верхнього рівня `initializers` у конфігурації вашого середовища. Одне додаткове з'єднання — це просто мапа з одним записом:

```yaml
initializers:
  multi_db:
    secondary_db:
      uri: postgres://loco:loco@localhost:5432/loco_app
      enable_logging: false
      connect_timeout: 500
      idle_timeout: 500
      min_connections: 1
      max_connections: 1
      auto_migrate: false
      dangerously_truncate: false
      dangerously_recreate: false
```

Додавайте нові записи, щоб відкрити більше з'єднань:

```yaml
initializers:
  multi_db:
    secondary_db:
      uri: postgres://loco:loco@localhost:5432/loco_app
      enable_logging: false
      connect_timeout: 500
      idle_timeout: 500
      min_connections: 1
      max_connections: 1
      auto_migrate: false
      dangerously_truncate: false
      dangerously_recreate: false
    third_db:
      uri: postgres://loco:loco@localhost:5432/loco_app_reporting
      enable_logging: false
      connect_timeout: 500
      idle_timeout: 500
      min_connections: 1
      max_connections: 1
      auto_migrate: false
      dangerously_truncate: false
      dangerously_recreate: false
```

## Зареєструйте ініціалізатор

```rust
use loco_rs::app::{AppContext, Initializer};

async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
    let initializers: Vec<Box<dyn Initializer>> = vec![
        Box::new(loco_rs::initializers::multi_db::MultiDbInitializer),
    ];

    Ok(initializers)
}
```

## Шукайте з'єднання за назвою

`MultiDbInitializer` додає `loco_rs::db::MultiDb` (тонку обгортку над `HashMap<String, DatabaseConnection>`) як axum `Extension`:

```rust
use sea_orm::EntityTrait;
use axum::{response::IntoResponse, Extension};
use loco_rs::db::MultiDb;

pub async fn list(
    State(ctx): State<AppContext>,
    Extension(multi_db): Extension<MultiDb>,
) -> Result<impl IntoResponse> {
    let third_db = multi_db.get("third_db")?;
    let res = Entity::find().all(third_db).await;
    format::json(res)
}
```

`multi_db.get(name)` повертає помилку, якщо такий ключ не налаштований — жодних мовчазних `None` чи панік.

## Результат

`ctx.db` залишається основним з'єднанням вашого застосунку (використовується для авто-міграції, перевірок під час завантаження тощо); додаткові з'єднання надходять суто через axum `Extension` `MultiDb` і лише в обробниках, які їх запитують.

## Міграція з `extra_db`

`ExtraDbInitializer` було видалено на користь `MultiDbInitializer`. Щоб перейти:

- Конфігурація: колишній `initializers.extra_db: { ... }` стає `initializers.multi_db: { <name>: { ... } }` — придумайте назву для вашого з'єднання і вкладіть ті самі ключі під нею.
- Реєстрація: замініть `Box::new(loco_rs::initializers::extra_db::ExtraDbInitializer)` на `Box::new(loco_rs::initializers::multi_db::MultiDbInitializer)`.
- Обробники: змініть `Extension(db): Extension<DatabaseConnection>` на `Extension(multi_db): Extension<MultiDb>`, а потім отримуйте з'єднання через `let db = multi_db.get("<name>")?;`.

## Далі

- [Довідник з конфігурації](/uk/docs/reference/configuration/) — кожен ключ, який приймає блок формату `database:`.
- [Додати middleware](/uk/docs/how-to/add-middleware/) — як працюють хуки `initializers`/middleware.
