---
title: Рендеринг серверних шаблонів
description: "Рендерте HTML за допомогою Tera-based ViewRenderer у Loco: створіть шаблон, підключіть екстрактор ViewEngine та використовуйте білдер format::render()."
sidebar:
  order: 12
---

**Мета:** повертати з контролера HTML, відрендерений на сервері, за допомогою вбудованого в Loco рушія шаблонів на основі [Tera](http://keats.github.io/tera/).

Це передбачає наявність робочого застосунку з налаштованим ініціалізатором рушія шаблонів (SaaS/HTML-стартери мають його з коробки). Якщо ви згенерували застосунок лише для API і додаєте HTML уперше, дивіться крок 5.

## 1. Створіть шаблон

Шаблони живуть під `assets/views/` (тека `assets/` розташована поруч із `src/` та `config/` у корені проєкту):

```html
<!-- assets/views/home/hello.html -->
<html>
  <body>
    <h1>{{ title }}</h1>
  </body>
</html>
```

## 2. Обгорніть його в типізовану функцію перегляду

Інкапсулюйте виклик шаблону, щоб контролери ніколи не торкалися Tera чи шляхів до шаблонів напряму — саме це дозволяє пізніше замінити рушій шаблонів, не зачіпаючи контролери.

```rust
// src/views/dashboard.rs
use loco_rs::prelude::*;

pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::render().view(&v, "home/hello.html", data!({"title": "Loco"}))
}
```

Додайте його до `src/views/mod.rs`:

```rust
pub mod dashboard;
```

## 3. Витягніть рушій шаблонів у своєму контролері

`ViewEngine<E>` — це екстрактор `FromRequestParts` — `TeraView` — це конкретний рушій, який постачає Loco. Обидва походять із prelude.

```rust
// src/controllers/dashboard.rs
use loco_rs::prelude::*;
use crate::views;

pub async fn render_home(ViewEngine(v): ViewEngine<TeraView>) -> Result<impl IntoResponse> {
    views::dashboard::home(v)
}

pub fn routes() -> Routes {
    Routes::new().prefix("home").add("/", get(render_home))
}
```

Зареєструйте маршрути контролера як завжди — дивіться [Додаємо контролер](/uk/docs/how-to/add-controller/).

`ViewEngine<E>` вимагає встановленого на роутері `Extension` типу `TeraLayer`; він панікує з `"TeraLayer missing. Is the TeraLayer installed?"`, якщо його немає. Це підключається через `ViewEngineInitializer` у `src/initializers/view_engine.rs` (присутній за замовчуванням у HTML/HTMX-стартерах) — дивіться крок 5, якщо потрібно його додати.

## 4. Два способи рендерингу

**`format::view`** — найпростіша форма, рендерить безпосередньо в HTML-відповідь:

```rust
pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::view(&v, "home/hello.html", data!({"title": "Loco"}))
}
```

**`format::render()`** — налаштовуваний ланцюжком білдер, коли потрібно більше, ніж голе тіло HTML зі статусом 200. Він завершується `.view(...)`, `.template(...)`, `.html(...)` або `.json(...)`, і може спочатку додати заголовки, `ETag`, cookies або код статусу:

```rust
pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::render()
        .etag("home-v1")?
        .cookies(&[axum_extra::extract::cookie::Cookie::new("last_view", "home")])?
        .view(&v, "home/hello.html", data!({"title": "Loco"}))
}
```

`format::render()` також має аварійний вихід `.response()`, який повертає базовий `axum::http::response::Builder`, якщо вам потрібно щось, чого ланцюжок не покриває, і `.redirect(to)` / `.redirect_with_header_key(key, to)` для перенаправлень (дивіться [Відповіді в різних форматах](/uk/docs/how-to/respond-formats/).

Для вбудованого рядка шаблону без файлу на диску використовуйте `format::template(tmpl, data)` (або `.template(...)` білдера) замість `.view(...)`.

## 5. Увімкнення рушія шаблонів у застосунку лише для API

Якщо ваш застосунок ще не підключає рушій шаблонів (headless/API-застосунки цього не роблять), додайте ініціалізатор:

```rust
// src/initializers/view_engine.rs
use async_trait::async_trait;
use axum::{Extension, Router as AxumRouter};
use loco_rs::{
    app::{AppContext, Initializer},
    controller::views::{engines, ViewEngine},
    Result,
};

pub struct ViewEngineInitializer;

#[async_trait]
impl Initializer for ViewEngineInitializer {
    fn name(&self) -> String {
        "view-engine".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let tera_engine = engines::TeraView::build()?;
        Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
    }
}
```

Зареєструйте його в `src/app.rs`:

```rust
async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
    Ok(vec![Box::new(initializers::view_engine::ViewEngineInitializer)])
}
```

`TeraView::build()` завантажує шаблони з `assets/views` (`DEFAULT_ASSET_FOLDER = "assets"`). Використовуйте натомість `TeraView::build_with_post_process(|tera| { ... })`, якщо потрібно зареєструвати власні функції Tera (наприклад, функцію i18n `t(...)`) — дивіться `src/initializers/view_engine.rs` демонстраційного застосунку для робочого прикладу з `fluent-templates`.

## 6. Обслуговування статичних ресурсів, на які посилаються ваші шаблони

Шаблони, що посилаються на `<img src="/static/...">`, потребують middleware `static` — дивіться [Обслуговування статичних та SPA-ресурсів](/uk/docs/how-to/serve-assets/).

## 7. Використайте зовсім інший рушій шаблонів

Оскільки контролери залежать лише від `ViewRenderer` (трейт з одним методом: `render<S: Serialize>(&self, key: &str, data: S) -> Result<String>`), ви можете замінити Tera на будь-що інше. Реалізуйте `ViewRenderer` для власного типу, зареєструйте його через `Initializer` так само, як `TeraView` вище, і замініть узагальнений параметр екстрактора (`ViewEngine<TeraView>` → `ViewEngine<YourEngine>`) — жодних змін у логіці контролера.

## Перевірка

```sh
cargo loco routes   # переконайтеся, що GET /home зареєстровано
curl -s localhost:5150/home
```

## Далі

- [Відповіді в різних форматах](/uk/docs/how-to/respond-formats/) — JSON/HTML/YAML та узгодження вмісту
- [Обслуговування статичних та SPA-ресурсів](/uk/docs/how-to/serve-assets/)
- [Обробка помилок](/uk/docs/how-to/handle-errors/)
