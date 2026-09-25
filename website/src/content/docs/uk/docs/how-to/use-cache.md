---
title: Використання кешу
description: Налаштуйте драйвер кешу (null/в пам'яті/Redis) та використовуйте get/insert/get_or_insert з терміном дії, ping і clear.
sidebar:
  order: 31
---

Мета: кешувати значення (обчислений результат, відповідь зовнішнього API, «гарячий» запит) за ключем, з опційним TTL, використовуючи драйвер, який можна замінювати між середовищами.

`ctx.cache` доступний у кожному контролері, задачі та воркері. Значення під капотом сериалізуються як JSON-рядки, тож усе, що реалізує `Serialize + DeserializeOwned`, можна покласти туди.

## 1. Налаштуйте драйвер

Драйвери кешу налаштовуються повністю в YAML — жодних змін коду для переключення драйверів між середовищами не потрібно.

```yaml
# config/development.yaml — швидко, одноразово, без зовнішньої залежності
cache:
  kind: InMem
  max_capacity: 33554432 # опційно, байти; стандартно 32MiB (32 * 1024 * 1024)
```

```yaml
# config/production.yaml — спільно між процесами
cache:
  kind: Redis
  uri: "<%= get_env(name='REDIS_CACHE_URL', default='redis://127.0.0.1:6379') %>"
  max_size: 10 # обов'язково — максимальна кількість з'єднань пулу
```

```yaml
# повністю пропустіть ключ `cache`, або встановіть явно — це стандартна поведінка
cache:
  kind: Null
```

`InMem` потребує фічі `cache_inmem` (увімкнена за замовчуванням); `Redis` потребує `cache_redis` (вимкнена за замовчуванням — додайте її до вашого `Cargo.toml`). Дивіться [довідник feature-прапорців](/uk/docs/reference/feature-flags/).

Якщо ви взагалі пропустите `cache` у конфігураційному файлі, Loco мовчки повернеться до драйвера **`Null`**: `get()` завжди повертає `None`, а кожна операція, що змінює стан (`insert`, `insert_with_expiry`, `remove`, `clear`, `ping`), повертає помилку. Це fail-fast поведінка за замовчуванням для випадку «ви не налаштували справжній кеш» — не випускайте її на production випадково.

## 2. Записуйте та читайте значення

```rust
use loco_rs::cache;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

async fn cache_basics(ctx: &AppContext) -> Result<()> {
    ctx.cache.insert("greeting", &"hello".to_string()).await?;

    let user = User { name: "Alice".to_string(), age: 30 };
    ctx.cache.insert("user:1", &user).await?;

    let greeting: Option<String> = ctx.cache.get("greeting").await?;
    let cached_user: Option<User> = ctx.cache.get("user:1").await?;

    let exists: bool = ctx.cache.contains_key("user:1").await?;

    ctx.cache.remove("greeting").await?;
    Ok(())
}
```

## 3. Встановіть TTL за допомогою `insert_with_expiry`

```rust
use std::time::Duration;

ctx.cache
    .insert_with_expiry("session:abc", &token, Duration::from_secs(300))
    .await?;
```

## 4. Кешуйте результат обчислення за допомогою `get_or_insert`

`get_or_insert` повертає кешоване значення, якщо воно є; інакше виконує заданий future, зберігає результат і повертає його. `get_or_insert_with_expiry` робить те саме, але додає TTL до свіжообчисленого значення.

```rust
let expensive_report = ctx
    .cache
    .get_or_insert::<Report, _>("report:daily", async {
        build_daily_report(ctx).await
    })
    .await?;
```

```rust
use std::time::Duration;

let expensive_report = ctx
    .cache
    .get_or_insert_with_expiry::<Report, _>(
        "report:daily",
        Duration::from_secs(3600),
        async { build_daily_report(ctx).await },
    )
    .await?;
```

## 5. Перевірка здоров'я та очищення

```rust
// Дає збій, якщо бекенд-сховище (напр. Redis) недоступне.
ctx.cache.ping().await?;

// Стерти кеш.
ctx.cache.clear().await?;
```

> **Застереження щодо Redis:** `Cache::clear()` на драйвері Redis виконує **`FLUSHDB`** — він очищає *всю* логічну базу даних Redis, а не лише ключі, які поклав туди ваш застосунок. Якщо інші дані (сховище сесій, черга, інший застосунок) спільно використовують ту саму БД/інстанс Redis, `clear()` видалить і їх. Направте кеш на власну БД Redis (`redis://host:6379/1`, окремий індекс `db`), якщо потрібна ізоляція, і ставтеся до `clear()` як до грубої операції над усією базою даних.

## 6. Перевірте

```rust
#[tokio::test]
async fn can_get_or_insert() {
    let app_ctx = get_app_context().await; // ваш тестовий AppContext
    let key = "loco";

    assert_eq!(app_ctx.cache.get::<String>(key).await.unwrap(), None);

    let result = app_ctx
        .cache
        .get_or_insert::<String, _>(key, async { Ok("loco-cache-value".to_string()) })
        .await
        .unwrap();

    assert_eq!(result, "loco-cache-value");
    assert_eq!(
        app_ctx.cache.get::<String>(key).await.unwrap(),
        Some("loco-cache-value".to_string())
    );
}
```

## Довідник

- Кожен YAML-ключ `cache:` (`kind`, `max_capacity`, `uri`, `max_size`): [довідник конфігурації § cache](/uk/docs/reference/configuration/#cache)
- Feature-прапорці `cache_inmem` / `cache_redis`: [довідник feature-прапорців](/uk/docs/reference/feature-flags/)
