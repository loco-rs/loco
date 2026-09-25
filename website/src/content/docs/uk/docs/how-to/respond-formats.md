---
title: Відповіді в різних форматах
description: "Використовуйте помічники відповідей format:: (json, text, html, yaml, redirect, empty) і узгоджуйте тип вмісту за допомогою RespondTo/Format."
sidebar:
  order: 13
---

**Мета:** повертати з обробника правильну форму відповіді (JSON, HTML, звичайний текст, YAML, перенаправлення, порожнє тіло), а — коли один ендпоінт має обслуговувати понад один формат — вибирати форму на основі заголовків `Content-Type`/`Accept` запиту.

Це передбачає наявність робочого контролера — дивіться [Додаємо контролер](/uk/docs/how-to/add-controller/). Усі помічники живуть у модулі `format` (`loco_rs::controller::format`, реекспортований як `format` з prelude). Нехай обробники повертають `Result<impl IntoResponse>` (або `Result<Response>`), тоді ви можете вільно змінювати, який саме виклик `format::*` повертаєте.

## 1. Прості відповіді

```rust
use loco_rs::prelude::*;

async fn as_json() -> Result<Response> {
    format::json(serde_json::json!({ "hello": "world" }))
}

async fn as_text() -> Result<Response> {
    format::text("hello, world")
}

async fn as_html() -> Result<Response> {
    format::html("<h1>hello</h1>")
}

async fn as_yaml() -> Result<Response> {
    format::yaml("openapi: 3.1.0\ninfo:\n  title: my api\n")
    // встановлює Content-Type: application/yaml
}

async fn nothing() -> Result<Response> {
    format::empty() // 200, порожнє тіло
}

async fn empty_object() -> Result<Response> {
    format::empty_json() // 200, тіло: {}
}

async fn go_elsewhere() -> Result<Response> {
    format::redirect("/dashboard") // axum::response::Redirect::to(..)
}
```

`format::view`/`format::template` рендерять HTML з Tera-шаблону або вбудованого рядка шаблону — дивіться [Рендеринг серверних шаблонів](/uk/docs/how-to/render-views/).

## 2. Коли потрібно більше, ніж один рядок: `format::render()`

`format::render()` повертає `RenderBuilder`, який можна налаштовувати ланцюжком, завершуючи одним із `.json(..)`, `.html(..)`, `.text(..)`, `.empty()`, `.view(..)`, `.template(..)`, `.redirect(..)` або `.redirect_with_header_key(..)`:

```rust
async fn get_one(State(ctx): State<AppContext>) -> Result<Response> {
    format::render()
        .etag("some-etag-value")?
        .header("X-Custom", "1")
        .json(load_item(&ctx).await?)
}
```

Доступні методи білдера:

| Метод | Призначення |
|---|---|
| `.status(code)` | встановити статус відповіді (за замовчуванням `200`) |
| `.header(key, value)` | додати один заголовок відповіді |
| `.etag(value)` | встановити заголовок `ETag` (помилка на не-ASCII-visible вхідних даних) |
| `.cookies(&[Cookie, ..])` | додати один заголовок `Set-Cookie` на кожну cookie |
| `.response()` | аварійний вихід: повернути сирий `axum::http::response::Builder` |
| `.redirect(to)` | `303 See Other` з `Location: <to>` |
| `.redirect_with_header_key(key, to)` | те саме, але з власним заголовком замість `Location` — наприклад, `HX-Redirect` для HTMX |

```rust
async fn htmx_redirect() -> Result<Response> {
    format::render().redirect_with_header_key("HX-Redirect", "/notes")
}
```

## 3. Узгодження вмісту: різні відповіді для різних клієнтів

Використовуйте екстрактор `RespondTo` (або його обгортку `Format(pub RespondTo)`), щоб визначити формат запиту з `Content-Type` (перевіряється першим) або, якщо його немає, з `Accept`:

```rust
use loco_rs::prelude::*;

pub async fn get_one(
    respond_to: RespondTo,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    match respond_to {
        RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
        _ => format::json(item),
    }
}
```

Варіанти `RespondTo`: `None` (жоден заголовок не присутній/не парситься), `Html`, `Json`, `Xml`, `Other(String)` (будь-який інший MIME-тип, збережений як є). І `RespondTo`, і `Format` реалізують `FromRequestParts`, тому ви можете витягти будь-який з них безпосередньо як параметр обробника — `Format(respond_to)`, якщо віддаєте перевагу обгорнутій формі.

## 4. Поєднуйте узгодження форматів з обробкою помилок

Поширений патерн: спочатку виконайте свою помилконебезпечну логіку, потім обробіть `Result` і формат в одному місці, щоб рендеринг помилок залишався послідовним для кожного формату:

```rust
pub async fn get_one(
    respond_to: RespondTo,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let res = load_item(&ctx, id).await;

    match res {
        Ok(item) => match respond_to {
            RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
            _ => format::json(item),
        },
        Err(Error::Model(ModelError::Validation(errors))) => match respond_to {
            RespondTo::Html => format::html(&format!("<html><body>errors: {errors:?}</body></html>")),
            _ => bad_request("opaque message: cannot respond!"),
        },
        // необроблені види помилок: нехай типовий рендеринг помилок фреймворку візьме верх
        Err(err) => Err(err),
    }
}
```

Дивіться [Обробка помилок](/uk/docs/how-to/handle-errors/), щоб дізнатися, що «типовий рендеринг помилок фреймворку» повертає для кожного варіанту `Error`.

## Перевірка

```sh
curl -s localhost:5150/notes/1 -H 'accept: application/json'
curl -s localhost:5150/notes/1 -H 'accept: text/html'
```

## Далі

- [Рендеринг серверних шаблонів](/uk/docs/how-to/render-views/)
- [Обробка помилок](/uk/docs/how-to/handle-errors/)
- [Валідація запитів](/uk/docs/how-to/validate-requests/)
