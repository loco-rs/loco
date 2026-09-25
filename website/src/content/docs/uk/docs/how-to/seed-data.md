---
title: Наповнення даними (seed)
description: "Завантажте фікстури до нової бази даних, підключіть їх до Hooks::seed та дампіть/імпортуйте вміст таблиць через cargo loco db seed."
sidebar:
  order: 4
---

**Мета:** наповнити щойно мігровану базу даних відомими рядками з YAML-файлів фікстур — для локальної розробки, тестів та відтворення даних одного середовища в іншому.

Це передбачає робочу модель (дивіться [Додати модель](/uk/docs/how-to/add-model/).

## 1. Напишіть файл фікстур

Фікстури живуть у `src/fixtures/`, один YAML-файл на таблицю, зі звичайним списком записів:

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

Включайте кожну колонку `NOT NULL`, визначену вашою міграцією; колонки, що допускають NULL, можна пропускати.

## 2. Підключіть фікстуру до `Hooks::seed`

Додайте виклик `db::seed::<ActiveModel>` до реалізації `Hooks::seed` вашого застосунку, по одному рядку на файл фікстур:

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

`db::seed` читає YAML у `Vec<serde_json::Value>`, перетворює кожен рядок через `A::from_json`, вставляє їх за допомогою `insert_many`, а потім скидає автоінкрементну послідовність таблиці, щоб рядки, створені згодом, не конфліктували з ідентифікаторами фікстур.

## 3. Запустіть команду наповнення

```sh
$ cargo loco db seed
```

Стандартно вона читає з `src/fixtures` та вставляє в те середовище, на яке ви націлюєтеся (`-e`/`--environment`, типово `development`). Поширені прапорці:

```sh
# очистити всі дані перед наповненням — зручно для повторюваного скидання dev/test
$ cargo loco db seed --reset

# наповнити з іншої теки (наприклад, фікстур для конкретного середовища)
$ cargo loco db seed --from src/fixtures/staging

# націлитися на конкретне середовище
$ cargo loco db seed -e test
```

## 4. Дампіть наявні дані назад у фікстури

Та сама команда працює й у зворотному напрямку: експортує чинний вміст таблиць у YAML-файли, наприклад, щоб зафіксувати знімок даних, подібних до продуктових, для локальних фікстур.

```sh
# дамп кожної таблиці до теки --from (типово: src/fixtures)
$ cargo loco db seed --dump

# дамп лише конкретних таблиць
$ cargo loco db seed --dump-tables users,posts
```

`--dump`/`--dump-tables` та наповнення взаємовиключні в межах одного виклику — передача будь-якого з них виконує дамп замість наповнення.

## 5. Використовуйте наповнення в тестах

З увімкненою можливістю `testing` комбінація `boot_test` + `seed::<App>` дає кожному тесту щойно наповнену базу даних:

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_find_seeded_user() {
    let boot = boot_test::<App>().await?;
    seed::<App>(&boot.app_context).await?;

    let user = Model::find_by_email(&boot.app_context.db, "user1@example.com").await;
    assert!(user.is_ok());
}
```

## Результат

`cargo loco db seed` (опційно з `--reset`) залишає вашу базу даних наповненою точно тими рядками з `src/fixtures/*.yaml`, а `cargo loco db seed --dump` дозволяє заново згенерувати ті самі файли фікстур з живої бази даних щоразу, коли змінюється ваша схема або демонстраційні дані.
