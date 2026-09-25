---
title: Каталог middleware
description: "Усі вбудовані middleware: ключ конфігурації, структура, стандартний стан та параметри."
sidebar:
  order: 5
---

Loco постачається з 13 вбудованими middleware, усі реалізують трейт
`MiddlewareLayer` (`src/controller/middleware/mod.rs:46-72`). Кожен
налаштовується під `server.middlewares.<key>` у YAML вашого середовища
(`src/config/server.rs:44`) і є опціональним (`Option<T>`) — повністю
опустіть ключ, щоб отримати власний стандарт фреймворку; вкажіть ключ (навіть
як `{}`), щоб натомість перейняти його `serde`-стандарти (див. примітку нижче).

## Трейт `MiddlewareLayer`

```rust
pub trait MiddlewareLayer {
    fn name(&self) -> &'static str;
    fn is_enabled(&self) -> bool { true } // default
    fn config(&self) -> serde_json::Result<serde_json::Value>;
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>>;
}
```

`src/controller/middleware/mod.rs:46-72`.

> **Наявність ключа конфігурації перевертає стандарт.** Для кожного middleware
> нижче, чий "стандартно увімкнений" стан дорівнює `true` (`catch_panic`,
> `etag`, `logger`, `request_id` та — поза Production — `fallback`), цей
> стандарт походить від власного резервного значення `default_middleware_stack`,
> яке використовується лише тоді, коли ключ **відсутній** у конфігурації
> (`Option` є `None`, `src/controller/middleware/mod.rs:76-171`). Якщо ви
> взагалі напишете ключ — навіть як порожнє відображення (`etag: {}`) —
> власний `#[serde(default)]` структури переймає керування, що розв'язує
> `enable` у `false`, якщо ви явно не встановите `enable: true`. Коротше:
> не пишіть ключ middleware у конфігурації, якщо ви не маєте наміру також
> встановити `enable`.

## Порядок стека: порядок побудови проти порядку запиту (LIFO)

`default_middleware_stack(ctx)` (`mod.rs:76-171`) повертає middleware як `Vec`
у порядку кодування нижче (limit_payload → … → powered_by).
`AppRoutes::to_router` застосовує їх у тому самому порядку, по одному виклику
`app.layer(...)` за раз (`src/controller/app_routes.rs:305-309`). Метод
`Router::layer` в Axum загортає *належний* роутер кожним новим шаром як
**зовнішнім** шаром, отже:

> "ОСТАННІЙ middleware є ПЕРШИМ, хто зустрічає зовнішній світ (запуск
> користувацького запиту), тобто порядок 'LIFO'" —
> `src/controller/app_routes.rs:283-286`.

Тож вхідний запит насправді проходить через стек у **зворотному** порядку
щодо таблиці — `powered_by` першим, `limit_payload` останнім (безпосередньо
перед обробником маршруту) — а відповідь тече назад протилежним шляхом.

## Повний набір middleware

Порядок у таблиці = порядок кодування/конфігурації
(`default_middleware_stack`, `mod.rs:80-169`).

### 1. `limit_payload`

- **Ключ конфігурації:** `limit_payload` · **Структура:** `limit_payload::LimitPayload` (`limit_payload.rs:26`)
- **Стандарт:** фактично завжди увімкнений — `is_enabled()` жорстко запрограмований як `true` (`limit_payload.rs:70-72`); поля `enable` немає. Щоб вимкнути, встановіть `body_limit: disable`.
- **Призначення:** обмежує розмір тіла запиту через `DefaultBodyLimit` в Axum.
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `body_limit` | `DefaultBodyLimitKind` (`"<size>"` напр. `"5mb"`, або `"disable"`) | `2mb` (2 000 000 байт) — `limit_payload.rs:43-45` |

### 2. `cors`

- **Ключ конфігурації:** `cors` · **Структура:** `cors::Cors` (`cors.rs:19-42`)
- **Стандарт:** **вимкнений**.
- **Призначення:** заголовки Cross-Origin Resource Sharing.
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `false` |
| `allow_origins` | `Vec<String>` | `["*"]` |
| `allow_headers` | `Vec<String>` | `["*"]` |
| `allow_methods` | `Vec<String>` | `["*"]` |
| `expose_headers` | `Vec<String>` | `[]` (порожньо) |
| `allow_credentials` | `bool` | `false` |
| `max_age` | `Option<u64>` (секунди) | `None` |
| `vary` | `Vec<String>` | `["origin", "access-control-request-method", "access-control-request-headers"]` |

> Поле називається `expose_headers` (у множині) — `cors.rs:32`.

### 3. `catch_panic`

- **Ключ конфігурації:** `catch_panic` · **Структура:** `catch_panic::CatchPanic { enable }` (`catch_panic.rs:18-22`)
- **Стандарт:** **увімкнений**.
- **Призначення:** перехоплює паніки в обробниках запитів, логує їх і повертає `500 Internal Server Error` замість розриву з'єднання.
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `true` (стандарт фреймворку, коли ключ відсутній) |

### 4. `etag`

- **Ключ конфігурації:** `etag` · **Структура:** `etag::Etag { enable }` (`etag.rs:27-31`)
- **Стандарт:** **увімкнений**.
- **Призначення:** порівнює `If-None-Match` з `ETag` відповіді та повертає `304 Not Modified` у разі збігу.
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `true` (стандарт фреймворку, коли ключ відсутній) |

### 5. `remote_ip`

- **Ключ конфігурації:** `remote_ip` · **Структура:** `remote_ip::RemoteIpMiddleware { enable, source }` (`remote_ip.rs`)
- **Стандарт:** **вимкнений**.
- **Призначення:** визначає IP клієнта з одного, довіреного джерела (заголовка проксі або сирої адреси сокета). Реалізовано як тонка обгортка над крейтом [`axum-client-ip`](https://docs.rs/axum-client-ip).
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `false` |
| `source` | `axum_client_ip::ClientIpSource` | `RightmostXForwardedFor` |

`source` обирає рівно одне довірене джерело — тут немає проходження ланцюжка
проксі та немає списку довірених CIDR. Дійсні значення (серіалізуються як
голе ім'я варіанта, напр. `source: XRealIp`): `RightmostXForwardedFor`
(останнє значення останнього заголовка `X-Forwarded-For`, взяте як є),
`RightmostForwarded` (заголовок `Forwarded` за RFC 7239), `CfConnectingIp`
(Cloudflare), `CloudFrontViewerAddress` (AWS CloudFront), `FlyClientIp`
(Fly.io), `TrueClientIp` (Akamai/Cloudflare), `XEnvoyExternalAddress`
(Envoy/Istio), `XRealIp` (nginx) або `ConnectInfo` (сира адреса піра сокета,
без жодного заголовка).

> **ЗМІНА, ЩО ЛАМАЄ СУМІСНІСТЬ (було `trusted_proxies: Option<Vec<String>>`):**
> старий middleware власноруч розбирав `X-Forwarded-For`, проходячи заголовок
> справа наліво та пропускаючи будь-яку IP-адресу в налаштовуваному списку
> довірених проксі CIDR (або вбудованому списку RFC-1918 + loopback) — тобто
> він міг бачити крізь ланцюжок з одного чи кількох довірених проксі. Нове
> поле `source` довіряє рівно **одному** стрибку і взагалі не застосовує
> CIDR-фільтрацію. Якщо у вас кілька стрибків (CDN → балансувальник → ingress),
> налаштуйте свій найвнутрішніший стрибок так, щоб він сам обчислював і
> встановлював правильну IP-адресу клієнта, і вкажіть `source` на той
> заголовок, який він записує (або оберіть джерело, специфічне для провайдера,
> як-от `CfConnectingIp`).

### 6. `compression`

- **Ключ конфігурації:** `compression` · **Структура:** `compression::Compression { enable }` (`compression.rs:14-18`)
- **Стандарт:** **вимкнений**.
- **Призначення:** стискає тіла відповідей (`tower_http::compression::CompressionLayer`).
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `false` |

### 7. `timeout_request`

- **Ключ конфігурації:** `timeout_request` · **Структура:** `timeout::TimeOut { enable, timeout }` (`timeout.rs:23-30`)
- **Стандарт:** **вимкнений**.
- **Призначення:** перериває запит і повертає `408 Request Timeout`, якщо він триває довше за `timeout`.
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `false` |
| `timeout` | `u64` (мілісекунди) | `5000` (`timeout.rs:38-40`) |

### 8. `static`

- **Ключ конфігурації:** `static` (поле Rust `static_assets`, `#[serde(rename = "static")]`, `mod.rs:197-199`) · **Структура:** `static_assets::StaticAssets` (`static_assets.rs:24-43`)
- **Стандарт:** **вимкнений**.
- **Призначення:** роздає теку статичних файлів з опціональним резервним файлом для SPA-маршрутизації.
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `false` |
| `must_exist` | `bool` | `true` |
| `folder.uri` | `String` | `"/static"` |
| `folder.path` | `PathBuf` | `"assets/static"` |
| `fallback` | `PathBuf` | `"assets/static/404.html"` |
| `precompressed` | `bool` | `false` (роздає `.gz`-варіанти, коли `true`) |
| `cache_control` | `Option<String>` | `None` (напр. `"max-age=31536000"`) |

> Під увімкненою можливістю `embedded_assets` це під час компіляції замінюється
> на `static_assets_embedded::StaticAssets` (`mod.rs:21-27`) — той самий ключ
> конфігурації (`"static"`) та сама поверхня параметрів, але ресурси вбудовані
> у бінарник замість читання з диска.

### 9. `secure_headers`

- **Ключ конфігурації:** `secure_headers` · **Структура:** `secure_headers::SecureHeader { enable, preset, overrides }` (`secure_headers.rs:78-86`)
- **Стандарт:** **вимкнений**.
- **Призначення:** вбудовує попередньо визначений набір заголовків безпеки (CSP, X-Frame-Options тощо), кожен з яких можна перевизначити окремо.
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `false` |
| `preset` | `String` | `"github"` (`secure_headers.rs:94-96`) — інші пресети: `owasp`, `empty` (`secure_headers.json`) |
| `overrides` | `Option<BTreeMap<String, String>>` | `None` |

### 10. `logger`

- **Ключ конфігурації:** `logger` · **Структура:** `logger::Config { enable }` → `logger::Middleware` через `logger::new(config, &env)` (`logger.rs:21-25, 36-42`)
- **Стандарт:** **увімкнений**.
- **Призначення:** логування запитів на основі `TraceLayer` (метод, URI, версія, user agent, ID запиту, середовище).
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `true` (стандарт фреймворку, коли ключ відсутній) |

### 11. `request_id`

- **Ключ конфігурації:** `request_id` · **Структура:** `request_id::RequestId { enable }` (`request_id.rs:28-32`)
- **Стандарт:** **увімкнений**.
- **Призначення:** гарантує, що кожен запит має заголовок `x-request-id` (санітизує вхідний або генерує UUID v4) та надає його обробникам як `LocoRequestId(String)` через `.get()` (`request_id.rs:63-72`).
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `true` (стандарт фреймворку, коли ключ відсутній) |

### 12. `fallback`

- **Ключ конфігурації:** `fallback` · **Структура:** `fallback::Fallback { enable, code, file, not_found }` (`fallback.rs:17-37`); `StatusCodeWrapper(pub StatusCode)` (`fallback.rs:15`)
- **Стандарт:** увімкнений **лише коли `environment != Production`** (`mod.rs:158-167`).
- **Призначення:** повертає відповідь для невідповідних маршрутів — файл, просте повідомлення або вбудований `fallback.html` — замість голої 404 від Axum.
- **Параметри:**

| Назва | Тип | Стандарт |
|---|---|---|
| `enable` | `bool` | `true` поза Production, `false` у Production (стандарт фреймворку, коли ключ відсутній) |
| `code` | `StatusCode` (як `u16`) | `404` (`NOT_FOUND`) — `default_status_code`, `fallback.rs:39-41`. Встановлюйте лише для незвичайного випадку, коли невідповідний маршрут має відповідати чимось іншим |
| `file` | `Option<String>` | `None` — шлях до файлу, що роздається як тіло фолбеку |
| `not_found` | `Option<String>` | `None` — просте текстове повідомлення, що роздається як тіло фолбеку |

Якщо не задано ані `file`, ані `not_found`, роздається вбудований `fallback.html`.

### 13. `powered_by`

- **Ключ конфігурації:** відсутній — **не** є частиною `middleware::Config`; керується через `server.ident: Option<String>` (`src/config/server.rs:40`). Структура: `powered_by::Middleware` через `powered_by::new(ctx.config.server.ident.as_deref())` (`powered_by.rs:27-58`)
- **Стандарт:** **увімкнений**, встановлює заголовок ідентифікації сервера `X-Powered-By: loco.rs`.
- **Призначення:** встановлює заголовок відповіді `X-Powered-By`.
- **Параметри (через `server.ident`, а не `enable`):**

| Значення `server.ident` | Ефект |
|---|---|
| відсутнє / `None` | `X-Powered-By: loco.rs` (стандарт) |
| `""` (порожній рядок) | middleware вимкнено — заголовка немає |
| будь-який інший рядок | `X-Powered-By: <рядок>` |

## Редагування стека: `MiddlewareStackExt`

`Hooks::middlewares` передає вам `Vec<Box<dyn MiddlewareLayer>>`, який
створив `default_middleware_stack`. Щоб підправити його, а не будувати з нуля,
трейт `MiddlewareStackExt` (`src/controller/middleware/mod.rs:187-198`)
додає чотири редагування в стилі Rails, реалізовані для цього `Vec` та
реекспортовані з `loco_rs::prelude` (`src/prelude.rs:39`), тож він уже в зоні
видимості в згенерованих застосунках:

| Метод | Ефект |
|---|---|
| `insert_before(name, middleware)` | Вставляє безпосередньо перед першим middleware, чиїм `name()` є `name` |
| `insert_after(name, middleware)` | Вставляє безпосередньо після нього |
| `replace(name, middleware)` | Замінює його на місці |
| `delete(name)` | Видаляє його |

Кожен метод повертає `&mut Self`, тож виклики ланцюжаться. Middleware
зіставляються за `MiddlewareLayer::name()`, а позиції — це позиції у `Vec`,
що, згідно з приміткою про LIFO вище, означає: "перед" у порядку `Vec` є
*пізніше* в порядку запиту.

Якщо нічого не відповідає `name`, операція логує попередження, а не зазнає
невдачі: `insert_before` / `insert_after` все одно додають новий middleware
в кінець (тож він не губиться мовчки), тоді як `replace` / `delete`
залишають стек недоторканим.

```rust
fn middlewares(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
    let mut stack = loco_rs::controller::middleware::default_middleware_stack(ctx);
    stack.delete("powered_by").insert_after("etag", Box::new(MyMiddleware));
    stack
}
```

## Інтроспекція стека

```
cargo loco middleware           # list every middleware and its enabled state
cargo loco middleware --config  # also print each middleware's JSON config
```

`src/cli.rs:100-104`, у поєднанні з `list_middlewares` (`src/boot.rs:588-597`),
який викликає `name()`, `is_enabled()` та `config()` кожного middleware.
