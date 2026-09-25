---
title: Додаємо middleware
description: Увімкніть вбудоване проміжне ПЗ (middleware) через YAML-конфігурацію або напишіть власне MiddlewareLayer, якщо вбудованих недостатньо.
sidebar:
  order: 15
---

**Мета:** увімкнути одне з 13 вбудованих middleware Loco або написати власне, якщо жодне не підходить, і переконатися, що воно справді працює.

Це передбачає наявність робочого застосунку. Повну таблицю ключів/параметрів конфігурації для кожного вбудованого middleware дивіться у [каталозі middleware](/uk/docs/reference/middleware/).

## 1. Увімкніть вбудоване middleware через конфігурацію

Кожне middleware живе під `server.middlewares.<ключ>` у YAML-файлі вашого середовища (`config/development.yaml`, `config/production.yaml`, ...). Більшість вимкнені за замовчуванням; кілька (`catch_panic`, `etag`, `logger`, `request_id` та `fallback` поза `Production`) увімкнені, якщо ви взагалі не пишете ключ.

Увімкніть `remote_ip` (корисно за проксі/балансувальником) та `compression`:

```yaml
server:
  middlewares:
    remote_ip:
      enable: true
    compression:
      enable: true
```

> **Обережно:** для middleware, увімкнених *за замовчуванням* (наприклад, `etag`, `catch_panic`), сам факт запису ключа — навіть як `{}` — замінює власний default фреймворку на власний `#[serde(default)]` структури, який розгортає `enable` у `false`, якщо ви явно не встановите `enable: true`. Не додавайте ключ middleware до конфігурації, якщо не маєте наміру також встановити `enable`.

## 2. Перевірте, що воно зареєстровано

```sh
cargo loco middleware --config
```

```sh
limit_payload          {"body_limit":{"Limit":2000000}}
cors                   (disabled)
catch_panic            {"enable":true}
etag                   {"enable":true}
remote_ip              {"enable":true,"source":"RightmostXForwardedFor"}
compression            {"enable":true}
timeout_request        (disabled)
static                 (disabled)
secure_headers         (disabled)
logger                 {"config":{"enable":true},"environment":"development"}
request_id             {"enable":true}
fallback               {"enable":true,"code":200,"file":null,"not_found":null}
powered_by             {"ident":"loco.rs"}
```

`cargo loco middleware` (без `--config`) виводить лише стан увімкнено/вимкнено.

## 3. Поширені приклади

Встановіть ліміт розміру тіла запиту:

```yaml
server:
  middlewares:
    limit_payload:
      body_limit: 5mb   # або "disable", щоб прибрати ліміт повністю
```

Увімкніть CORS (вимкнений за замовчуванням — зверніть увагу, поле називається `expose_headers`, у **множині**):

```yaml
server:
  middlewares:
    cors:
      enable: true
      allow_origins:
        - https://example.com
      allow_headers:
        - Content-Type
      allow_methods:
        - GET
        - POST
      expose_headers:
        - X-Custom-Header
      max_age: 3600
```

Обслуговуйте статичні ресурси або SPA — повний покроковий посібник дивіться у [Обслуговування статичних та SPA-ресурсів](/uk/docs/how-to/serve-assets/):

```yaml
server:
  middlewares:
    static:
      enable: true
      folder:
        uri: "/static"
        path: "assets/static"
```

Потім використайте екстрактор для middleware, який його надає, наприклад `RemoteIP`:

```rust
use loco_rs::prelude::*;

#[debug_handler]
pub async fn list(ip: RemoteIP, State(ctx): State<AppContext>) -> Result<Response> {
    tracing::info!(%ip, "handling request");
    format::json(Entity::find().all(&ctx.db).await?)
}
```

## 4. Застосуйте middleware лише до одного маршруту замість глобального

Middleware, керовані конфігурацією, завжди застосовуються до кожного маршруту застосунку. Щоб обмежити `tower::Layer` одним контролером чи маршрутом, використовуйте `Routes::layer` — дивіться [Додаємо контролер § 7](/uk/docs/how-to/add-controller/#7-застосуйте-towerlayer-лише-до-одного-контролера-чи-маршруту).

## 5. Напишіть власне middleware

Реалізуйте трейт `MiddlewareLayer`:

```rust
pub trait MiddlewareLayer {
    fn name(&self) -> &'static str;
    fn is_enabled(&self) -> bool { true } // за замовчуванням
    fn config(&self) -> serde_json::Result<serde_json::Value>;
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>>;
}
```

Мінімальний приклад, який додає до кожної відповіді власний заголовок і вмикається/вимикається з конфігурації, як вбудовані:

```rust
// src/middlewares/hello.rs
use axum::{http::HeaderValue, response::Response, Router as AXRouter};
use loco_rs::{app::AppContext, controller::middleware::MiddlewareLayer, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HelloHeader {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for HelloHeader {
    fn name(&self) -> &'static str {
        "hello_header"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(axum::middleware::map_response(add_header)))
    }
}

async fn add_header(mut res: Response) -> Response {
    res.headers_mut()
        .insert("X-Hello", HeaderValue::from_static("loco"));
    res
}
```

Зареєструйте його поруч (або замість) типового стеку, перевизначивши хук `middlewares` на `App` у `src/app.rs`:

```rust
impl Hooks for App {
    // ...
    fn middlewares(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
        let mut mids = middleware::default_middleware_stack(ctx);
        mids.push(Box::new(middlewares::hello::HelloHeader { enable: true }));
        mids
    }
}
```

Пам'ятайте правило порядку: `AppRoutes::to_router` застосовує цей `Vec` по одному виклику `.layer(...)` за раз, і кожен новий шар загортає роутер як **зовнішній** шар — тому middleware **останній** у vec є **першим**, хто бачить вхідний запит (LIFO). Додавайте власне middleware до того кінця vec, який відповідає його бажаному положенню відносно `logger`/`catch_panic` тощо. Повне пояснення та порядок вбудованих дивіться у [каталозі middleware § порядок стеку](/uk/docs/reference/middleware/#порядок-стека-порядок-побудови-проти-порядку-запиту-lifo).

## Перевірка

```sh
cargo loco middleware --config   # переконайтеся, що ваш власний запис і його конфігурація з'явилися
curl -i localhost:5150/          # переконайтеся, що власний заголовок/поведінка проявляються
```

## Далі

- [Каталог middleware](/uk/docs/reference/middleware/) — ключ конфігурації та параметри кожного вбудованого middleware
- [Обслуговування статичних та SPA-ресурсів](/uk/docs/how-to/serve-assets/)
- [Обробка помилок](/uk/docs/how-to/handle-errors/)
