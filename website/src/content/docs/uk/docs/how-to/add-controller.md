---
title: Додаємо контролер
description: Згенеруйте контролер, налаштуйте його маршрути та обробники і змонтуйте їх під префіксом або вкладеним шляхом.
sidebar:
  order: 10
---

**Мета:** додати нову групу HTTP-ендпоінтів до вашого Loco-застосунку — згенерованих або написаних вручну — і побачити їх у `cargo loco routes`.

Цей посібник передбачає, що у вас є робочий Loco-застосунок (`cargo loco start` запускається). Повний API `Routes`/`AppRoutes` та вичерпну поверхню `Hooks` дивіться у [довіднику Hooks](/uk/docs/reference/hooks/).

## 1. Згенеруйте контролер

```sh
cargo loco generate controller <NAME> [ACTION ...]
```

Контролери генеруються як контролери JSON API — прапорця для вибору типу немає. Додаткові позиційні аргументи стають додатковими actions (функціями-обробниками + маршрутами) поруч із типовим `index`.

```sh
cargo loco generate controller notes list show
```

Уникайте називати action іменем HTTP-методу (`get`, `post`, `delete`, ...): згенерований файл робить `use loco_rs::prelude::*`, що підтягує `axum::routing::get` та інші, і обробник з таким самим ім'ям затінить функцію маршрутизації, потрібну згенерованій `routes()`.

Це:

- створює `src/controllers/notes.rs` з обробником `index` плюс по одному обробнику на кожен додатковий action (`list`, `show`), кожен повертає `format::empty()` як відправну точку
- додає `pub mod notes;` до `src/controllers/mod.rs`
- вбудовує `.add_route(controllers::notes::routes())` у вашу реалізацію `routes()` у `src/app.rs` — ручне підключення не потрібне
- генерує відповідний тестовий файл під `tests/requests/`

Згенерована функція `routes()` виглядає так:

```rust
// src/controllers/notes.rs
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/notes/")
        .add("/", get(index))
        .add("list", get(list))
        .add("show", get(show))
}
```

Відредагуйте тіла обробників і методи маршрутів (`get`/`post`/`put`/`delete` тощо) відповідно до вашого ендпоінта. Контролери за замовчуванням повертають JSON; якщо ви хочете рендерити HTML на сервері, дивіться [Рендеринг серверних шаблонів](/uk/docs/how-to/render-views/).

## 2. Перевірте, що маршрути зареєстровано

```sh
cargo loco routes
```

```sh
[GET] /_ping
[GET] /_health
[GET] /_readiness
[GET] /api/notes/
[GET] /api/notes/list
[GET] /api/notes/show
```

Якщо нові маршрути не з'являються, перевірте, що реалізація `routes()` у `src/app.rs` викликає `.add_route(controllers::notes::routes())` (генератор робить це за вас, але перепровірте після ручного редагування чи конфлікту злиття).

## 3. Напишіть контролер вручну (без генератора)

Іноді потрібен контролер без скаффолда від генератора — наприклад, невеликий внутрішній ендпоінт.

1. Створіть `src/controllers/example.rs`:

   ```rust
   use loco_rs::prelude::*;

   async fn hello() -> Result<Response> {
       format::text("hello")
   }

   async fn echo(Json(body): Json<serde_json::Value>) -> Result<Response> {
       format::json(body)
   }

   pub fn routes() -> Routes {
       Routes::new()
           .add("/", get(hello))
           .add("/echo", post(echo))
   }
   ```

2. Оголосіть модуль у `src/controllers/mod.rs`:

   ```rust
   pub mod example;
   ```

3. Зареєструйте його маршрути в `Hooks::routes` у `src/app.rs`:

   ```rust
   fn routes(_ctx: &AppContext) -> AppRoutes {
       AppRoutes::with_default_routes()
           .add_route(controllers::example::routes())
   }
   ```

`AppRoutes::with_default_routes()` також монтує вбудовані ендпоінти моніторингу `/_ping`, `/_health`, `/_readiness`.

## 4. Префікс для цілого контролера

`Routes::prefix` застосовує префікс до кожного маршруту, доданого до цього екземпляра `Routes`:

```rust
pub fn routes() -> Routes {
    Routes::new()
        .prefix("notes")
        .add("/", get(list))
        .add("/{id}", get(get_one))
}
```

## 5. Префікс для цілого застосунку (або групи контролерів)

`AppRoutes::prefix` застосовується до кожного контролера, доданого після нього:

```rust
fn routes(_ctx: &AppContext) -> AppRoutes {
    AppRoutes::with_default_routes()
        .prefix("/api")
        .add_route(controllers::notes::routes())
        .add_route(controllers::users::routes())
}
```

## 6. Вкладайте маршрути в додатковий сегмент шляху

Використовуйте `nest_prefix`, щоб додати ще один сегмент шляху до *поточного* префікса для маршрутів, доданих після нього, або `nest_route`/`nest_routes`, щоб застосувати префікс лише до переданих маршрутів (не змінюючи поточний префікс):

```rust
fn routes(_ctx: &AppContext) -> AppRoutes {
    let v1_notes = Routes::new().add("/", get(|| async { "notes v1" }));

    AppRoutes::with_default_routes()
        .prefix("api")
        .add_route(controllers::auth::routes())
        // лише ці маршрути отримають додатковий сегмент `v1`: /api/v1/...
        .nest_route("v1", v1_notes)
}
```

`Routes::nest` (на значенні `Routes`, а не `AppRoutes`) робить те саме, коли ви компонуєте групи маршрутів перед поверненням їх із функції `routes()` контролера — зручно для об'єднання кількох підресурсів через `Routes::merge`/`merge_all` з подальшим одним вкладенням результату:

```rust
let user_routes = Routes::new()
    .add("/users", get(list_users))
    .add("/users", post(create_user));
let product_routes = Routes::new().add("/products", get(list_products));

let api_routes = Routes::new().merge(user_routes).merge(product_routes);

Routes::new()
    .add("/health", get(|| async { "ok" }))
    .nest("/api", api_routes);
// -> GET /health, GET /api/users, POST /api/users, GET /api/products
```

## 7. Застосуйте `tower::Layer` лише до одного контролера чи маршруту

`Routes::layer` прикріплює `tower::Layer` (обмеження частоти запитів, власна автентифікація, трасування тощо) лише до кожного обробника в цьому значенні `Routes` — для middleware, що має виконуватися на *кожному* маршруті, дивіться натомість [Додаємо middleware](/uk/docs/how-to/add-middleware/).

```rust
// src/controllers/notes.rs
pub fn routes() -> Routes {
    Routes::new()
        .prefix("notes")
        .add("/", get(list).layer(my_tower_layer()))
}
```

## Перевірка

```sh
cargo loco routes
cargo test --test requests_notes   # якщо генератор створив tests/requests/notes.rs
```

Запит `curl` до нового шляху має повернути відповідь вашого обробника:

```sh
curl -s localhost:5150/api/notes/
```

## Далі

- [Валідація запитів](/uk/docs/how-to/validate-requests/)
- [Відповіді в різних форматах](/uk/docs/how-to/respond-formats/)
- [Обробка помилок](/uk/docs/how-to/handle-errors/)
