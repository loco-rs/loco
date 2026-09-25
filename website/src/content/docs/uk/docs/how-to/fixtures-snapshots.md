---
title: Знімкові тести з фікстурами та редагуваннями
description: Використовуйте insta-знімки для моделей і відповідей, редагуйте динамічні поля за допомогою cleanup_user_model/cleanup_email та робіть твердження щодо відрендереного HTML через select().
sidebar:
  order: 52
---

Мета: зробити знімок моделі, відповіді запиту або відрендереного HTML за допомогою [insta](https://crates.io/crates/insta), без того, щоб знімок «стрибав» під час кожного запуску через свіжий UUID, часову мітку чи хеш пароля.

## 1. Увімкніть insta

```toml
[dev-dependencies]
loco-rs = { version = "*", features = ["testing"] }
insta = { version = "*", features = ["redactions"] }
```

Loco **не** реекспортує `insta` — це звичайна dev-залежність вашого застосунку. Loco надає *таблиці фільтрів* (`src/testing/redaction.rs`), які ви передаєте в налаштування `filters` insta.

## 2. Зробіть знімок значення за допомогою `assert_debug_snapshot!`

```rust
use insta::assert_debug_snapshot;
use loco_rs::testing::prelude::*;

let boot = boot_test::<App>().await.unwrap();
let user = users::Model::find_by_email(&boot.app_context.db, "user1@example.com").await;

assert_debug_snapshot!(user);
```

Під час першого запуску файл `.snap` записується до теки `snapshots/` поруч із вашим тестом; перегляньте та прийміть його за допомогою `cargo insta review` (або вручну), і наступні запуски порівнюватимуться з ним.

## 3. Редагуйте динамічні поля перед створенням знімка

Щойно створений користувач має випадковий UUID `pid`, зростаючий `id`, bcrypt-хеш `password` і часові мітки `created_at`/`updated_at` — усе це змінюється під час кожного запуску і зламало б знімок. Загорніть твердження в `insta::with_settings!` з одним із наборів фільтрів очищення Loco:

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

| Функція фільтра | Що редагує | Заповнювач |
|---|---|---|
| `cleanup_user_model()` | UUID/PID, bcrypt-хеші `password: "..."`, JWT-подібні токени, часові мітки ISO-8601 (з часовим поясом і без), `id: <number>` | `PID`, `"PASSWORD"`, `TOKEN`, `DATE`, `id: ID` |
| `cleanup_email()` | Ідентифікатори повідомлень мейлера, дати RFC-2822, випадкові id у формі UUID | `IDENTIFIER`, `DATE`, `RANDOM_ID` |

Обидва поєднують базову таблицю зі спільним фільтром дат (`get_cleanup_date()`); `cleanup_user_model()` додатково вбудовує `get_cleanup_model()` (правило `id: N` → `id: ID`). Якщо вам потрібна власна комбінація, окремі таблиці (`get_cleanup_user_model()`, `get_cleanup_date()`, `get_cleanup_model()`, `get_cleanup_mail()`) також є публічними — побудуйте з них власний список фільтрів `Vec<(&str, &str)>`.

Використовуйте `cleanup_email()` так само під час створення знімків доставок мейлера (`ctx.mailer.unwrap().deliveries()`):

```rust
with_settings!({
    filters => cleanup_email()
}, {
    assert_debug_snapshot!(ctx.mailer.unwrap().deliveries());
});
```

## 4. Дайте кожному тестовому файлу власний простір назв знімків

Назви файлів знімків походять від назви тестової функції — для різних тестових файлів цього зазвичай достатньо, але якщо два файли мають тест з однаковою назвою, або ви просто хочете неймспейс на файл, визначте невеликий локальний макрос (це *не* API Loco — це звичайний insta, один рядок склеювального коду, повторюваний у кожному тестовому файлі):

```rust
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("users"); // додає суфікс до кожного знімка в цьому файлі
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

`cargo loco generate model`/`scaffold` уже створюють цей макрос (без рядка суфікса) у згенерованому `tests/models/<name>.rs` — дивіться [Використання генераторів](/uk/docs/how-to/use-generators/).

## HTML-твердження за допомогою `select()`

Для server-rendered переглядів (HTML/HTMX) не робіть знімків і не порівнюйте рядки із сирою розміткою — розбирайте її за допомогою хелперів селекторів Loco на основі `scraper` (`src/testing/selector.rs`). Усі вони панікують із зрозумілим повідомленням у разі невдачі, тож читаються як звичайні твердження:

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

`select(html, selector) -> Vec<String>` повертає зовнішній HTML кожного збігу — для випадків, коли ви хочете зробити знімок фрагмента замість твердження щодо нього напряму (поєднуйте з `assert_debug_snapshot!` і наведеними вище фільтрами редагування, якщо фрагмент містить динамічні дані):

```rust
let items = select(&html, ".item");
assert_debug_snapshot!(items);
```

## Перевірте це

```sh
cargo test
```

Щоб інтерактивно переглядати/оновлювати знімки після навмисних змін виводу, встановіть і запустіть [`cargo-insta`](https://crates.io/crates/cargo-insta):

```sh
cargo install cargo-insta
cargo insta review
```

Без встановленого `cargo-insta` та саму роботу виконує одна змінна середовища — вона перезаписує кожен застарілий знімок на місці, тому перегляньте отриманий `git diff` перед комітом:

```sh
INSTA_UPDATE=always cargo test
```

Знімки, що захоплюють усю модель (`assert_debug_snapshot!(user)`), дають збій за *будь-якої* зміни схеми, включно з суто адитивною. Це працює як задумано — знімок виконує свою роботу — але це означає, що додавання колонки до таблиці, яку знімають ваші тести, зламає кілька тестів одразу. Прийміть їх знову в тому самому коміті, що й міграцію, або звузьте твердження до полів, про які тест справді йдеться.
