---
title: Написання запит-тестів (тестів контролерів)
description: Запустіть тестовий екземпляр вашого застосунку та пройдіться по його HTTP-маршрутах за допомогою request/boot_test, RequestConfigBuilder і тверджень axum-test.
sidebar:
  order: 50
---

Мета: викликати HTTP-ендпоінти вашого застосунку з тесту — без реального зайнятого порту — і зробити твердження щодо відповіді, використовуючи хелпери Loco `request`/`boot_test` поверх [axum-test](https://crates.io/crates/axum-test).

## 1. Увімкніть фічу `testing`

Запит-тести потребують Cargo-фічі `testing` (вона підтягує `axum-test`, `scraper` і `tree-fs`). Додайте її до `dev-dependencies` — у застосунках, згенерованих `loco new`, вона вже є:

```toml
[dev-dependencies]
loco-rs = { version = "*", features = ["testing"] }
serial_test = "*"
insta = { version = "*", features = ["redactions"] }
```

## 2. Напишіть запит-тест із `request::<App, _, _>()`

`request` запускає ваш застосунок у `Environment::Test` і дає вам `TestServer` у пам'яті разом із `AppContext` застосунку — база даних не створюється:

```rust
use demo::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn can_get_notes() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/notes/").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}
```

Колбек отримує `(TestServer, AppContext)` — `request` (це `TestServer` з [axum-test](https://crates.io/crates/axum-test)) виконує HTTP-виклики (`.get`, `.post`, `.json(...)` тощо), а `ctx` дає вам той самий `AppContext`, який бачать ваші контролери (з'єднання з БД, конфіг, мейлер, ...) — дивіться [довідник AppContext](/uk/docs/reference/app-context/).

Позначайте тести як `#[serial]` (з крейту `serial_test`), щоразу коли вони спільно використовують стан на рівні застосунку або БД з іншими тестами, щоб вони не виконувалися конкурентно над тими самими фікстурами.

Якщо вашому тесту потрібна реальна, щойно створена база даних (зареєструвати користувача, а потім прочитати його назад), використовуйте натомість `request_with_create_db` — дивіться [тест моделей із БД](/uk/docs/how-to/model-tests/), щоб дізнатися більше про тести з підтримкою БД та очищення.

## 3. Робіть твердження щодо відповіді

`request`/`response` походять безпосередньо з axum-test — типові твердження:

```rust
let response = request.post("/api/auth/login").json(&payload).await;

assert_eq!(response.status_code(), 200);
response.assert_json(&serde_json::json!({ "token": "..." }));
let body: LoginResponse = serde_json::from_str(&response.text()).unwrap();
```

Для HTML/HTMX-відповідей розбирайте `response.text()` за допомогою [тверджень HTML-селекторів](/uk/docs/how-to/fixtures-snapshots/#html-твердження-за-допомогою-select) (`assert_css_exists`, `assert_css_eq`, `select`, ...) замість порівняння рядків із сирою розміткою.

## 4. Налаштуйте запит за допомогою `RequestConfigBuilder`

`request` використовує `RequestConfig` за замовчуванням (без збережених cookie, `default_content_type: "application/json"`). Щоб це змінити — наприклад, зберігати cookie між викликами в межах одного тесту (потоки авторизації через cookie/сесію) — побудуйте власну конфігурацію та використайте `request_with_config`:

```rust
use loco_rs::testing::prelude::*;

let config = RequestConfigBuilder::new()
    .save_cookies(true)
    .default_content_type("application/json")
    .build();

request_with_config::<App, _, _>(config, |request, _ctx| async move {
    // cookie, встановлені одним викликом, надсилаються в наступних викликах у межах цього замикання
    request.post("/session/login").json(&payload).await;
    let res = request.get("/session/me").await;
    assert_eq!(res.status_code(), 200);
})
.await;
```

Методи `RequestConfigBuilder`: `.save_cookies(bool)`, `.default_content_type(impl Into<String>)`, `.default_scheme(impl Into<String>)`, `.build()`.

> **Підводний камінь:** `RequestConfig::default_scheme` *не* передається до нижчого `TestServerConfig` axum-test — пересилаються лише `default_content_type` і `save_cookies`. Встановлення `.default_scheme(...)` сьогодні не має жодного видимого ефекту; не покладайтеся на нього, щоб примусово отримати `https`.

## 5. Звертайтеся до `boot_test` безпосередньо, коли HTTP вам не потрібен

`request` — це тонка обгортка: вона викликає `boot_test::<App>()`, щоб запустити застосунок, а потім загортає роутер у `TestServer`. Якщо вам узагалі не потрібно проходити через HTTP — наприклад, ви тестуєте модель або сервісну функцію безпосередньо — викличте `boot_test` самостійно і пропустіть сервер:

```rust
use demo::app::App;
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn test_something() {
    let boot = boot_test::<App>().await.expect("failed to boot test app");
    // використовуйте boot.app_context, напр. boot.app_context.db, boot.app_context.config, ...
}
```

`boot_test<H: Hooks>()` є **одногенеричним** — він приймає лише вашу реалізацію `Hooks` (зазвичай `App`), а не другий параметр типу `Migrator`. Сигнатури знаходяться в `src/testing/request.rs`:

| Функція | Коли використовувати |
|---|---|
| `boot_test::<App>()` | Потрібен `AppContext` без БД і без HTTP. |
| `boot_test_with_create_db::<App>()` | Потрібен `AppContext` на основі **свіжої одноразової бази даних** (лише з with-db). Дивіться [тест моделей із БД](/uk/docs/how-to/model-tests/). |
| `boot_test_unique_port::<App>(port)` | Потрібен сервер, фактично прив'язаний до TCP-порту (рідкісний випадок — більшість тестів мають використовувати натомість `request`/`TestServer`). |

`request`/`boot_test` обидва запускаються з `Environment::Test`, тому читають `config/test.yaml` — дивіться [пріоритет завантаження конфігурації](/uk/docs/reference/configuration/#завантаження-та-пріоритет), щоб дізнатися, як розв'язуються тека конфігурації та назва середовища.

## Перевірте це

```sh
cargo test
```

Успішний тест виводить звичайне `test result: ok` від `cargo test`; невдале твердження щодо статус-коду чи JSON панікує зі стандартним diff/повідомленням axum-test.
