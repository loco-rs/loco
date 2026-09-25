---
title: Модель помилок
description: Enum Error у loco-rs, його відображення на HTTP-статуси, JSON-тіло ErrorDetail та конструктори wrap/msg/string/bt.
sidebar:
  order: 8
---

`loco-rs` використовує єдиний тип помилки на весь crate. Ця сторінка документує його структуру, те, як він перетворюється на HTTP-відповідь, та помічники для його конструювання.

## `Result` та `Error`

- `pub type Result<T, E = Error> = std::result::Result<T, E>` (`src/lib.rs:52`).
- `pub use self::errors::Error` (`src/lib.rs:5`) — реекспортується в корені crate'а та в `loco_rs::prelude`.
- `Error` оголошено з атрибутом `#[non_exhaustive]` (`src/errors.rs:31`). Будь-який `match Error { .. }` поза межами crate'а **зобов'язаний** мати wildcard-гілку `_ =>` — компілятор це вимагає. Всередині самого `loco_rs` атрибут `#[non_exhaustive]` не діє, і наведена нижче таблиця статусів навмисно не використовує wildcard.

## Відображення варіантів на HTTP-статуси

`impl IntoResponse for Error` (`src/controller/mod.rs:198`) зіставляє помилку і створює `(StatusCode, ErrorDetail)`, а потім серіалізує `ErrorDetail` як JSON-тіло відповіді. Match **навмисно вичерпний — гілки `_ =>` немає** (`src/controller/mod.rs:274-284`): кожен внутрішній/інфраструктурний варіант перелічено на ім'я в гілці, що відповідає 500, тому додавання нового варіанту `Error` будь-де в crate'і є помилкою компіляції тут, доки хтось не вирішить, який статус він має нести, — замість того, щоб мовчки ставати 500.

| Варіант | Коли виникає | HTTP-статус | `ErrorDetail` у відповіді |
|---|---|---|---|
| `NotFound` | Повертається помічником `not_found()` або безпосередньо. | 404 | `{"error":"not_found","description":"Resource was not found"}` |
| `Unauthorized(String)` | Повертається помічником `unauthorized(msg)` або безпосередньо. Також логує `tracing::warn!(err)` з оригінальним повідомленням (саме повідомлення клієнту **не** надсилається). | 401 | `{"error":"unauthorized","description":"You do not have permission to access this resource"}` |
| `CustomError(StatusCode, ErrorDetail)` | Конструюється безпосередньо, коли викликач хоче довільний статус і тіло. | passthrough — той `StatusCode`, який було передано | passthrough — той `ErrorDetail`, який було передано |
| `WithBacktrace { inner, backtrace }` | Обгортає інший варіант помилки; створюється викликом `.bt()` на `Error` (backtrace збирається лише коли встановлено `RUST_BACKTRACE` — див. Конструктори нижче). Також виводить внутрішню помилку (червоним, підкреслено) та відфільтрований backtrace у stdout через `backtrace::print_backtrace`. | passthrough — власний статус обгорнутої помилки | passthrough — власне тіло обгорнутої помилки. `mod.rs:238-245` повторно викликає `into_response` на `*inner`, тому збір backtrace ніколи не змінює HTTP-результат (внутрішня помилка лишається 500, а не спрощується до 400) |
| `BadRequest(String)` | Повертається помічником `bad_request(msg)` або безпосередньо. | 400 | `{"error":"Bad Request","description":"<msg>"}` |
| `JsonRejection(JsonRejection)` | Extractor `Json` в axum відхиляє некоректне/відсутнє тіло запиту (проявляється через `#[from_request(rejection(Error))]` обгортки `Json<T>`). Логує `tracing::debug!(err = err.body_text(), ...)`. | `err.status()` — власний статус відхилення axum (зазвичай 400/415/422) | `{"error":"Bad Request"}` |
| `AxumFormRejection(FormRejection)` | Extractor `Form` в axum відхиляє некоректне тіло запиту. Логує `tracing::debug!(err = err.body_text(), ...)` (`mod.rs:254-257`). | `err.status()` — власний статус відхилення axum | `{"error":"Bad Request"}` |
| `Validation(ModelValidationErrors)` | Невдача валідації з crate `validator`, конвертована через `#[from] ModelValidationErrors`. | 400 | `{"errors": <serde_json::Value помилок полів>}` — зауважте, тут `error`/`description` дорівнюють `None`; заповнюється лише `errors` |
| `Model(ModelError::EntityNotFound)` | Пошук у моделі не знайшов жодного рядка (`with-db`, `mod.rs:261-265`). | 404 | `{"error":"not_found","description":"Resource was not found"}` |
| `Model(ModelError::EntityAlreadyExists)` | Вставка в модель конфліктує з наявним рядком (`with-db`, `mod.rs:266-270`). | 409 | `{"error":"conflict","description":"Resource already exists"}` |
| `Model(ModelError::TenantMismatch)` | `TenantActiveModelExt::set_tenant` намагався перезаписати ключ іншого tenant'а (feature `multi-tenancy`). | 400 | `{"error":"tenant_mismatch","description":"Model belongs to a different tenant"}` |
| `Model(ModelError::Validation(..))` | Невдача валідації на рівні моделі (`with-db`, `mod.rs:271-272`); те саме тіло, що й у верхньорівневої `Validation`. | 400 | `{"errors": <serde_json::Value помилок полів>}` |
| **усі решта варіантів**, кожен перелічений на ім'я в match | Внутрішні/інфраструктурні варіанти (`DB`, `IO`, `Redis`, `Sqlx`, `Tera`, `YAML`, `Message`, `Any`, `InternalServerError`, решта варіантів `Model` — `DbErr`, `Any`, `Message`, `Jwt` — тощо; повний список див. нижче) | **500** | `{"error":"internal_server_error","description":"Internal Server Error"}` |

Кожна відповідь, незалежно від варіанту, спочатку логується через `tracing::error!` з полями `error.msg` / `error.details` до того, як виконається match (`src/controller/mod.rs:184-202`).

## `ErrorDetail` — форма тіла відповіді

```rust
#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
}
```
(`src/controller/mod.rs:133-142`)

Конструктори:

| Функція | Сигнатура | Поведінка |
|---|---|---|
| `ErrorDetail::new` | `new<T1, T2>(error: T1, description: T2) -> Self` (`:147`) | Встановлює `error`; встановлює `description` у `None`, якщо переданий опис — порожній рядок, інакше `Some(..)`. `errors` завжди `None`. |
| `ErrorDetail::with_reason` | `with_reason<T>(error: T) -> Self` (`:161`) | Встановлює лише `error`; `description`/`errors` дорівнюють `None`. |

Тіло завжди загортається у власний тип crate'а `Json<T>` (`src/controller/mod.rs:170-178`, тонкий newtype над `axum::Json`), а не в сирий `axum::Json`.

## Помічники-конструктори на `Error`

`src/errors.rs:153-177`:

| Функція | Сигнатура | Примітки |
|---|---|---|
| `Error::wrap` | `wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self` (`:154`) | Загортає будь-яку помилку в `Error::Any(Box::new(err))`. **Не** викликає `.bt()` (збір backtrace закоментовано). |
| `Error::msg` | `msg(err: impl std::error::Error + Send + Sync + 'static) -> Self` (`:158`) | Перетворює `Display` помилки на рядок у `Error::Message(err.to_string())`. Також не збирає backtrace. |
| `Error::string` | `string(s: &str) -> Self` (`:162`) | Будує `Error::Message(s.to_string())` безпосередньо зі стрічки. `#[must_use]`. |
| `Error::bt` | `bt(self) -> Self` (`:166`) | Збирає `std::backtrace::Backtrace::capture()`. Якщо статус backtrace — `Disabled` або `Unsupported` (тобто `RUST_BACKTRACE` не встановлено), повертає `self` без змін — без алокацій, без обгортання. Інакше загортає `self` у `Error::WithBacktrace`. `#[must_use]`. |

Обидва — `wrap`/`msg` — це дешеві помічники конвертації для перетворення сторонньої `std::error::Error` у `Error` crate'а в місці виклику (наприклад, всередині обробника через `.map_err(Error::wrap)`); `bt` — це opt-in-обгортка з backtrace, яка використовується внутрішньо (наприклад, вручну написана реалізація `From<serde_json::Error>` у `src/errors.rs:24-28` робить `Self::JSON(val).bt()`).

## Помічні функції контролерів

Вільні функції у `src/controller/mod.rs` для типових HTTP-варіантів, кожна повертає `Result<U>` (тобто завжди `Err(..)`):

| Функція | Сигнатура | файл:рядок |
|---|---|---|
| `unauthorized` | `unauthorized<T: Into<String>, U>(msg: T) -> Result<U>` | `:112` |
| `bad_request` | `bad_request<T: Into<String>, U>(msg: T) -> Result<U>` | `:121` |
| `not_found` | `not_found<T>() -> Result<T>` | `:130` |

Усі три реекспортуються з `loco_rs::prelude`.

## Повний список варіантів

Повний `#[non_exhaustive] enum Error` (`src/errors.rs:32-151`) з feature-прапорцями, де вони є:

| Варіант | Feature-прапорець |
|---|---|
| `WithBacktrace { inner: Box<Self>, backtrace: Box<Backtrace> }` | — |
| `Message(String)` | — |
| `QueueProviderMissing` | — |
| `TaskNotFound(String)` | — |
| `Scheduler(#[from] crate::scheduler::Error)` | — |
| `Axum(#[from] axum::http::Error)` | — |
| `Tera(#[from] tera::Error)` | — |
| `JSON(serde_json::Error)` | — (власноруч написаний `From`, а не `#[from]`, щоб можна було викликати `.bt()`) |
| `JsonRejection(#[from] JsonRejection)` | — |
| `YAMLFile(#[source] serde_yaml::Error, String)` | — |
| `YAML(#[from] serde_yaml::Error)` | — |
| `EmailSender(#[from] lettre::error::Error)` | — |
| `Smtp(#[from] smtp::Error)` | — |
| `Worker(String)` | — |
| `IO(#[from] std::io::Error)` | — |
| `DB(#[from] sea_orm::DbErr)` | `with-db` |
| `ParseAddress(#[from] AddressError)` | — |
| `Unauthorized(String)` | — |
| `NotFound` | — |
| `BadRequest(String)` | — |
| `CustomError(StatusCode, ErrorDetail)` | — |
| `InternalServerError` | — |
| `InvalidHeaderValue(#[from] InvalidHeaderValue)` | — |
| `InvalidHeaderName(#[from] InvalidHeaderName)` | — |
| `InvalidMethod(#[from] InvalidMethod)` | — |
| `Model(#[from] crate::model::ModelError)` | `with-db` |
| `Redis(#[from] redis::RedisError)` | `worker_redis` |
| `Sqlx(#[from] sqlx::Error)` | `worker` |
| `Storage(#[from] crate::storage::StorageError)` | — |
| `Cache(#[from] crate::cache::CacheError)` | — |
| `Generators(#[from] loco_gen::Error)` | `debug_assertions` |
| `VersionCheck(#[from] depcheck::VersionCheckError)` | — |
| `Any(#[from] Box<dyn std::error::Error + Send + Sync>)` | — |
| `Validation(#[from] ModelValidationErrors)` | — |
| `AxumFormRejection(#[from] axum::extract::rejection::FormRejection)` | — |

## Видалені варіанти (breaking-зміни у звуженні error-enum 1.0)

Комміт `4a4a84ee` («narrow the Error enum — drop 4 low-value/leaky variants») видалив чотири варіанти, які **підтверджено відсутні** в поточному коді (`src/errors.rs`):

- `EnvVar(#[from] std::env::VarError)`
- `Hash(String)`
- `SemVer(#[from] semver::Error)`
- `TaskJoinError(#[from] tokio::task::JoinError)`

Будь-який код, який конструює або робить `match` по цих чотирьох, більше не компілюється. У поєднанні з `#[non_exhaustive]` кожен downstream `match Error { .. }` має мати гілку `_ =>` — це вимагалося й до видалення, але видалення нагадує, що нові/видалені варіанти не мають ламати вичерпні match, а код користувача не повинен покладатися на вичерпність.
