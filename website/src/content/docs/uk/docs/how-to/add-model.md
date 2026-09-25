---
title: Додати модель
description: Згенеруйте модель з міграцією, додайте до неї колонки та виконайте міграцію, щоб отримати робочі сутності.
sidebar:
  order: 1
---

**Мета:** додати до застосунку Loco нову модель на основі бази даних — міграцію, сутність Sea-ORM та власний файл моделі для її розширення — за допомогою генератора моделей.

Це передбачає робочий застосунок Loco з увімкненою можливістю `with-db` (стандартно). Повний опис мінімови типів полів та всіх видів генераторів дивіться у [Generators & field types](/uk/docs/reference/generators/). Про DSL міграцій, що використовується під капотом, читайте у [Schema & ColType DSL](/uk/docs/reference/schema-dsl/).

## 1. Згенеруйте модель

Запустіть генератор моделей із назвою та списком пар `field:type`:

```sh
$ cargo loco generate model posts title:string! content:text user:references
```

Це робить три речі за один крок:

1. Записує міграцію до `migration/src/`, яка створює таблицю `posts`.
2. Застосовує міграцію до вашої бази даних розробки.
3. Регенерує сутності Sea-ORM до `src/models/_entities/` і створює `src/models/posts.rs` для вашого власного коду моделі.

У результаті ви отримаєте:

```
src/
  models/
    _entities/
      posts.rs   <-- згенерована сутність (Entity, Model, ActiveModel, Column, Relation)
    posts.rs     <-- точка розширення для вашого коду
migration/
  src/
    m20240101_000002_posts.rs
```

Встановіть змінну середовища `SKIP_MIGRATION`, якщо хочете, щоб генератор лише записав файл міграції, не застосовуючи її та не регенеруючи сутності — це зручно, коли ви у скрипті викликаєте кілька `generate model` поспіль, а `db migrate` запускаєте один раз наприкінці.

## 2. Розберіться з синтаксисом полів

Кожна пара `field:type` дотримується простої конвенції суфіксів:

- без суфікса → колонка, що допускає NULL (`Option<T>`)
- `!` → обов'язкова колонка (`NOT NULL`)
- `^` → унікальна колонка (передбачає `NOT NULL`)

Тож `title:string!` — це обов'язковий `String`, а `content:text` — `Option<String>`, який може бути порожнім.

`user:references` — особливий випадок: він не називає тип колонки, а оголошує зовнішній ключ типу belongs-to. Він додає обов'язкову колонку `user_id`, що посилається на таблицю `users` (`user:references?` робить її такою, що допускає NULL; `user:references:author_id` задає власну назву колонки). Повний синтаксис дивіться у [Generators & field types § References](/uk/docs/reference/generators/#references-зовнішні-ключі-belongs-to).

<div class="infobox">
Зміна в 1.0: тип поля <code>int</code>/<code>int!</code>/<code>int^</code> у генераторі тепер відповідає <b>i64 / BIGINT</b> (<code>big_integer</code>), а не <code>i32</code>, як у попередніх версіях Loco — це узгоджується з автоінкрементними первинними ключами i64 у фреймворку. Якщо вам справді потрібна 16-бітна колонка, використовуйте <code>small_int</code>.
</div>

## 3. Додайте колонки до наявної моделі

Щоб додати колонки до вже створеної таблиці, згенеруйте звичайну міграцію замість нової моделі — назвіть її за схемою `Add<Columns>To<Table>`, щоб Loco розпізнав міграцію «додавання колонок»:

```sh
$ cargo loco generate migration AddViewsToPosts views:int
```

Застосуйте її та регенеруйте сутності:

```sh
$ cargo loco db migrate
$ cargo loco db entities
```

Видалення колонок слідує дзеркальній конвенції іменування, `Remove<Columns>From<Table>`:

```sh
$ cargo loco generate migration RemoveViewsFromPosts views:int
```

Одразу після `db entities` очікуйте кілька невдалих тестів: тести згенерованого застосунку роблять знімки цілих моделей (`assert_debug_snapshot!(user)`), тож додана колонка змінює їхній вивід. Прийміть нові знімки в тому самому коміті, що й міграція — дивіться [Fixtures & snapshots](/uk/docs/how-to/fixtures-snapshots/#перевірте-це).

## 4. Генерація без часових міток (опційно)

Стандартно кожна таблиця, згенерована через DSL, отримує колонки `created_at`/`updated_at`. Щоб відмовитися від них, передайте `--without-tz` до `model`, `migration` або `scaffold`:

```sh
$ cargo loco generate model posts title:string! content:text --without-tz
```

<div class="infobox">
Прапорець називається <code>--without-tz</code>, а не <code>--without-timestamps</code> — старіше написання більше не працює.
</div>

## 5. Перевірте

Переконайтеся, що міграцію застосовано та сутності існують:

```sh
$ cargo loco db status
$ ls src/models/_entities/
```

Потім пишіть код безпосередньо проти моделі — наприклад, у скрипті `cargo playground` або тесті:

```rust
use loco_rs::testing::prelude::*;
use myapp::app::App;
use myapp::models::_entities::posts;

let boot = boot_test::<App>().await?;
let post = posts::ActiveModel {
    title: sea_orm::ActiveValue::set("hello".to_string()),
    user_id: sea_orm::ActiveValue::set(1),
    ..Default::default()
}
.insert(&boot.app_context.db)
.await?;

assert_eq!(post.title, "hello");
```

**Результат:** у вашій базі даних існує таблиця `posts`, `posts::Entity`/`Model`/`ActiveModel` компілюються, а `src/models/posts.rs` — це місце, куди ви додаєте власні методи (наприклад, `Model::find_by_title`) так само, як `examples/demo/src/models/users.rs` розширює згенеровану сутність `users`.

## Далі

- [Запит даних](/uk/docs/how-to/query-data/) за допомогою DSL `ConditionBuilder`.
- [Зовнішньоключові зв'язки](/uk/docs/reference/schema-dsl/#операції-рівня-таблиць) та [валідація запитів/моделей](/uk/docs/how-to/validate-requests/) за межами однієї таблиці.
