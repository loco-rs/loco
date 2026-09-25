---
title: Написання тестів моделей із БД
description: Запустіть тестовий застосунок з підтримкою бази даних через boot_test, завантажте фікстури та отримайте автоматичне очищення БД після кожного тесту через Drop у BootResultWrapper.
sidebar:
  order: 51
---

Мета: перевірити ваші Sea-ORM-моделі на реальній базі даних у тесті, із завантаженими фікстурами та подальшим очищенням бази даних — без жодного ручного коду завершення.

Ця сторінка охоплює частину `testing`-фічі, що стосується `with-db` (`db.rs`): `boot_test`, `seed` та дві стратегії життєвого циклу БД, які підтримує Loco. Для тестів на рівні HTTP дивіться [запит-тести](/uk/docs/how-to/request-tests/); для знімків/редагувань insta дивіться [Фікстури та знімки](/uk/docs/how-to/fixtures-snapshots/).

## 1. Увімкніть фічі

```toml
[dev-dependencies]
loco-rs = { version = "*", features = ["testing"] }
serial_test = "*"
```

`with-db` (увімкнена за замовчуванням) також має бути активною у вашій залежності `loco-rs`, щоб хелпери з `db.rs` (`seed`, `boot_test_with_create_db`, `TestSupport`, ...) були скомпільовані.

## 2. Оберіть стратегію життєвого циклу БД

Loco підтримує два способи запуску тестів із БД, і обидва є легітимними — обирайте залежно від того, чи ваш тестовий набір спільно використовує одну тестову базу даних, чи створює одноразову базу для кожного тесту.

### A. Спільна тестова БД + очищення перед запуском (що генерує `loco new`)

Направте `database.uri` з `config/test.yaml` на одну тестову базу даних (файл SQLite або БД Postgres) і встановіть `dangerously_truncate: true`. Тоді кожен виклик `boot_test::<App>()` очищає налаштовані таблиці перед виконанням тіла тесту, тож кожен тест стартує з чистого аркуша — за умови, що тести виконуються по одному:

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
#[serial] // обов'язково: тести спільно використовують одну БД, тому вони не можуть чергуватися
async fn can_find_by_pid() {
    let boot = boot_test::<App>().await.expect("failed to boot test app");
    seed::<App>(&boot.app_context).await.expect("failed to seed");

    let existing = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111").await;
    assert!(existing.is_ok());
}
```

> ⚠️ `dangerously_truncate` очищає дані під час кожного виклику `boot_test` — ніколи не направляйте її на базу даних, яка вам дорога (ніколи на production). Контролюйте, які саме таблиці очищаються, через хук `truncate` у вашій реалізації `Hooks` (`async fn truncate(ctx: &AppContext) -> Result<()> { truncate_table(&ctx.db, users::Entity).await?; Ok(()) }` — `truncate_table` повертає `Result<(), DbErr>`, тож їй потрібен `?`).

### B. Свіжа унікальна база даних для кожного тесту (`#[serial]` для ізоляції БД не потрібен)

`boot_test_with_create_db::<App>()` створює абсолютно нову базу даних з унікальною назвою перед запуском і повертає `BootResultWrapper` замість звичайного `BootResult`:

```rust
use demo::app::App;
use loco_rs::testing::prelude::*;

#[tokio::test]
async fn can_register_in_isolated_db() {
    let boot = boot_test_with_create_db::<App>()
        .await
        .expect("failed to boot test app with a fresh db");

    // BootResultWrapper deref-иться до BootResult, тож `boot.app_context` працює як завжди
    seed::<App>(&boot.app_context).await.expect("failed to seed");
    // ... працюйте з boot.app_context.db ...

} // <- `boot` видаляється тут: одноразова база даних очищається автоматично
```

Спосіб надання унікальної БД залежить від схеми `config.database.uri` (диспетчеризація в `init_test_db_creation`):

| Схема URI | Стратегія реалізації |
|---|---|
| `postgres://` | Створює нову базу даних з назвою `_loco_test_{10-char-random}_{unix-timestamp}` на тому ж сервері Postgres. |
| `sqlite://` | Підтримує БД через тимчасовий файл `tree-fs` (`test.sqlite` у свіжій тимчасовій теці). |
| усе інше | Пропуск без дій (`Any`) — використовується як є, без ізоляції. |

### `BootResultWrapper` самостійно очищає через `Drop`

`BootResultWrapper` (присутній лише з `with-db`) обгортає `BootResult` плюс `Box<dyn TestSupport>`, який створив одноразову БД:

- Він `Deref`-иться до `BootResult`, тож `boot.app_context`, `boot.router` тощо працюють точно так само, як звичайне значення, що повертає `boot_test`.
- Його реалізація `Drop` викликає `test_db.cleanup_db()` — для Postgres це `DROP DATABASE` одноразової БД у spawned blocking task; для SQLite він видаляє тимчасову теку. Це виконується автоматично в кінці області видимості тестової функції, без явного виклику завершення.

> **Застереження:** якщо тестовий процес буде вбито посеред виконання (наприклад, `Ctrl+C`), `Drop` ніколи не спрацює, і одноразова база даних/схема залишиться — її доведеться видалити вручну (`DROP DATABASE _loco_test_...` для Postgres або видалити залишену тимчасову теку для SQLite).

## 3. Завантажте фікстури

`seed::<App>(&ctx)` завантажує фікстури з жорстко закодованої теки `src/fixtures`, делегуючи вашій реалізації `Hooks::seed`:

```rust
seed::<App>(&boot.app_context).await.expect("failed to seed");
```

Це той самий формат фікстур, який використовує `cargo loco db seed` — дивіться [довідник CLI `db seed`](/uk/docs/reference/cli/#22-підкоманди-db), щоб дізнатися про структуру на диску та прапорець `--from <DIR>`, якщо ви зберігаєте фікстури в іншому місці.

## 4. Згенеровані тести моделей уже роблять це за вас

`cargo loco generate model <name> ...` (і `scaffold`) створюють стартовий тест у `tests/models/<name>.rs`, який уже слідує шаблону A вище — `boot_test::<App>()`, `seed::<App>()`, локальний макрос `configure_insta!()` для найменування знімків і закоментований виклик `assert_debug_snapshot!`, який ви заповните. Дивіться [Використання генераторів](/uk/docs/how-to/use-generators/).

## Перевірте це

```sh
cargo test
```

Для шаблону A запускайте весь набір (або принаймні тести моделей) разом, щоб `#[serial]` міг виконати свою роботу — запуск одного тесту ізольовано (`cargo test can_find_by_pid`) теж працює, але змішування serial і не-serial тестів із БД в одному бінарнику без `#[serial]` на всіх із них призведе до нестабільних збоїв через конкурентне очищення.
