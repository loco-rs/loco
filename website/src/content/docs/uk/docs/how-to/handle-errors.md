---
title: Обробка помилок
description: Повертайте unauthorized/bad_request/not_found з обробника, створюйте CustomError з довільним статусом і знайте, який статус-код фреймворк надсилає для всіх інших випадків.
sidebar:
  order: 14
---

**Мета:** повертати з обробника правильний HTTP-статус та JSON-тіло помилки, не пишучи `impl IntoResponse` власноруч.

Це передбачає наявність робочого контролера — дивіться [Додаємо контролер](/uk/docs/how-to/add-controller/). Кожен обробник, що повертає `loco_rs::Result<T>` (тобто `Result<T, Error>`), отримує автоматичне перетворення своєї помилки на HTTP-відповідь через `impl IntoResponse for Error` — вам ніколи не потрібно викликати `.into_response()` на помилці самостійно. Вичерпний список варіантів і конструкторів дивіться у [довіднику моделі помилок](/uk/docs/reference/errors/).

## 1. Три помічники найпоширеніших випадків

`loco_rs::prelude` реекспортує три вільні функції для HTTP-орієнтованих варіантів помилок, до яких ви найчастіше вдаватиметеся:

```rust
use loco_rs::prelude::*;

async fn get_one(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let Some(item) = find_item(&ctx, id).await? else {
        return not_found();
    };
    format::json(item)
}

async fn login(State(ctx): State<AppContext>, Json(params): Json<LoginParams>) -> Result<Response> {
    let Ok(user) = find_user(&ctx, &params.email).await else {
        return unauthorized("invalid credentials");
    };
    // ...
    format::json(user)
}

async fn create(Json(params): Json<CreateParams>) -> Result<Response> {
    if params.title.is_empty() {
        return bad_request("title is required");
    }
    // ...
    format::empty()
}
```

Кожна повертає `Result<U>` (завжди гілку `Err`), тому `return unauthorized(msg)` перевіряється типами проти будь-якого типу повернення обробника `Result<Response>`. Усі три також є просто звичайними способами створити `Error` і пробросити його через `?` з допоміжної функції, яку ви викликаєте з обробника.

| Функція | HTTP-статус | Тіло відповіді | Примітки |
|---|---|---|---|
| `not_found()` | 404 | `{"error":"not_found","description":"Resource was not found"}` | Не приймає повідомлення. |
| `unauthorized(msg)` | 401 | `{"error":"unauthorized","description":"You do not have permission to access this resource"}` | `msg` логується (`tracing::warn!`), але **не** надсилається клієнту. |
| `bad_request(msg)` | 400 | `{"error":"Bad Request","description":"<msg>"}` | `msg` **надсилається** клієнту. |

## 2. Усе інше падає у 500

`Error` є `#[non_exhaustive]` з приблизно 28 іншими варіантами (`DB`, `Model`, `IO`, `Tera`, `Message`, `InternalServerError`, ...). Лише сім варіантів отримують конкретний статус; match в `IntoResponse` фреймворку закінчується wildcard-гілкою — **кожен інший варіант стає `500 Internal Server Error`** з тілом `{"error":"internal_server_error","description":"Internal Server Error"}`.

На практиці це означає: якщо ви пробросите через `?` `sea_orm::DbErr`, `std::io::Error` чи будь-що інше, що перетворюється на `Error` через `#[from]`, і не обробите його явно, клієнт отримає узагальнену 500 — що зазвичай саме те, що потрібно (не розголошуйте внутрішні деталі), і кожна відповідь спочатку логується на `tracing::error!` незалежно від варіанту, тож справжню причину ви все одно побачите на сервері.

Повна таблиця варіант → статус, включно з `Validation` (→ 400, з крейту `validator`) та `JsonRejection` (→ власний статус відхилення axum), є у [довіднику моделі помилок](/uk/docs/reference/errors/).

## 3. Поверніть довільний статус: `Error::CustomError`

Коли жоден із вбудованих помічників не підходить — `409 Conflict`, `429 Too Many Requests`, форма тіла, яку фреймворк не генерує — створіть його безпосередньо:

```rust
use loco_rs::prelude::*;
use loco_rs::controller::ErrorDetail;
use axum::http::StatusCode;

async fn create(Json(params): Json<CreateParams>) -> Result<Response> {
    if already_exists(&params).await? {
        return Err(Error::CustomError(
            StatusCode::CONFLICT,
            ErrorDetail::new("conflict", "a resource with this name already exists"),
        ));
    }
    format::empty()
}
```

`Error::CustomError(StatusCode, ErrorDetail)` пропускає і статус, і тіло **без змін** — це єдиний варіант, який мапінг відповідей не перезаписує. `ErrorDetail::new(error, description)` встановлює обидва поля (порожній description згортається в `None`); `ErrorDetail::with_reason(error)` встановлює лише `error`. Тіло відповіді — завжди `{error, description, errors}` з полями `None`, пропущеними в JSON.

## 4. Перетворіть сторонню помилку без окремого варіанту

Використовуйте `Error::wrap`/`Error::msg` у місці виклику `?`/`.map_err(..)`, щоб згорнути будь-який `std::error::Error` у `Error::Any`/`Error::Message` (обидва падають у catch-all 500 вище):

```rust
let parsed: MyType = serde_json::from_str(&raw).map_err(Error::wrap)?;
```

Звертайтеся до `Error::string("...")`, коли у вас є просто `&str`/повідомлення без помилки-джерела для обгортання.

## 5. Обробка помилок з урахуванням content-type

Якщо ендпоінт має рендерити помилки по-різному для HTML- та JSON-клієнтів, обробіть у одному місці і `Result` помилконебезпечного виклику, і узгоджений формат — дивіться [Відповіді в різних форматах](/uk/docs/how-to/respond-formats/#4-поєднуйте-узгодження-форматів-з-обробкою-помилок).

Зверніть увагу на межу: це узгодження — те, що робить *ваш обробник*. `Error`, що виходить з обробника, рендериться власним `IntoResponse` фреймворку, який завжди повертає JSON (`{"error": ..., "description": ...}`) незалежно від заголовка `Accept` запиту. Якщо HTML-клієнт має побачити HTML-сторінку помилки, ловіть `Result` в обробнику, як описано вище, а не давайте помилці пробратися назовні.

## Перевірка

```sh
curl -i localhost:5150/notes/999999   # -> 404 {"error":"not_found",...}
curl -i -X POST localhost:5150/auth/login -d '{"email":"x","password":"y"}' -H 'content-type: application/json'
# -> 401 {"error":"unauthorized",...}
```

## Далі

- [Валідація запитів](/uk/docs/how-to/validate-requests/) — варіант `Validation` і структуровані помилки по полях
- [Відповіді в різних форматах](/uk/docs/how-to/respond-formats/)
- [Довідник моделі помилок](/uk/docs/reference/errors/) — повний список варіантів і конструкторів
