---
title: AppContext та prelude
description: "Структура AppContext поле за полем, а також усе, що імпортує use loco_rs::prelude::* в область видимості."
sidebar:
  order: 6
---

`AppContext` — це клоновувана структура, сумісна з `State` в `axum`, яка несе в собі всі спільні ресурси Loco-застосунку (з'єднання з БД, кеш, чергу, mailer, сховище, конфігурацію та відкритий DI-слот). `loco_rs::prelude` — це поверхня єдиного імпорту, який використовує код застосунку замість явного вказування окремих шляхів `loco_rs` та `axum`. Обидва оголошені у `src/app.rs` та `src/prelude.rs`.

## `AppContext`

Оголошено у `src/app.rs:253-273`:

```rust
#[derive(Clone, FromRef)]
#[non_exhaustive]
pub struct AppContext {
    pub environment: Environment,
    #[cfg(feature = "with-db")]
    pub db: DatabaseConnection,
    pub queue_provider: Option<Arc<bgworker::Queue>>,
    pub config: Config,
    pub mailer: Option<EmailSender>,
    pub storage: Arc<Storage>,
    pub cache: Arc<cache::Cache>,
    pub shared_store: Arc<SharedStore>,
}
```

### Derive-атрибути

`#[derive(Clone, FromRef)]` (`src/app.rs:253`). `FromRef` (з `axum`) автоматично генерує `impl FromRef<AppContext> для <FieldType>` для кожного поля, тому обробник може витягнути окреме поле безпосередньо — наприклад, `State<DatabaseConnection>` або `State<Arc<cache::Cache>>` — замість того, щоб завжди приймати весь `State<AppContext>`.

### Поля

| Поле | Тип | Feature-прапорець | Призначення |
|---|---|---|---|
| `environment` | `Environment` | немає | Профіль, з яким завантажився застосунок (`Production` / `Development` / `Test` / `Any(String)`); визначає вибір файлу конфігурації. |
| `db` | `DatabaseConnection` (Sea-ORM 2.0) | `#[cfg(feature = "with-db")]` | Пул з'єднань Sea-ORM, який використовується кожним запитом сутностей та міграціями `db::converge`. Повністю відсутній у структурі, коли `with-db` вимкнено. |
| `queue_provider` | `Option<Arc<bgworker::Queue>>` | немає | Черга фонових завдань (Redis / Postgres / SQLite / внутрішньопроцесна), якщо застосунок було запущено з підключеною чергою. `None` для застосунків без черги. |
| `config` | `Config` | немає | Повністю завантажена, десеріалізована конфігурація `config/<environment>.yaml` (+ накладка `.local.yaml`). |
| `mailer` | `Option<EmailSender>` | немає | Налаштований бекенд надсилання електронної пошти (SMTP або заглушка), якщо застосунок його увімкнув. |
| `storage` | `Arc<Storage>` | немає | Абстракція файлового/об'єктного сховища (локальний диск або хмарний бекенд, обраний через feature-прапорці `storage_*`). |
| `cache` | `Arc<cache::Cache>` | немає | Дескриптор кешу (у пам'яті, Redis або null-бекенд залежно від прапорців `cache_*` / `CacheConfig`). |
| `shared_store` | `Arc<SharedStore>` | немає | Контейнер DI з ключами `TypeId`, безпечний для конкурентного доступу (на основі `DashMap`) для зберігання довільних сервісів, визначених застосунком — див. нижче. |

`db` — єдине поле, яке виключається під час компіляції (а не просто стає `None`), коли його feature (`with-db`) вимкнено — всі інші поля присутні безумовно, а роль «не налаштовано» виконують `Option`/порожнє значення за замовчуванням.

### Створення: `builder` / `into_builder`

`AppContext` має атрибут `#[non_exhaustive]` (`src/app.rs:255`), тому код застосунку не може написати структурний літерал для нього і не може використовувати синтаксис функціонального оновлення (`AppContext { storage, ..ctx }`). Додавання нового поля у майбутньому релізі, отже, не є breaking change. Йому замінюють два конструктори (`src/app.rs:284-413`):

- `AppContext::builder(environment, db, config)` — або `builder(environment, config)` без `with-db` — повертає `AppContextBuilder`. Необхідні компоненти передаються як аргументи; `queue_provider`, `mailer`, `storage`, `cache` та `shared_store` — це методи-сеттери, а `build()` заповнює ті з них, що лишилися невстановленими, no-op-значенням за замовчуванням (null-драйвер сховища, null-кеш, порожній спільний store).
- `ctx.into_builder()` перетворює наявний контекст назад у builder, переносячи **кожен** компонент. Саме цей метод має використовувати [`Hooks::after_context`](/uk/docs/reference/hooks/): старт з чистого `AppContext::builder` компілюється, але мовчки втрачає все, що завантаження вже поклало на контекст (mailer, провайдер черги, кеш, спільний store), тоді як round-trip замінює один компонент і зберігає решту.

```rust
async fn after_context(ctx: AppContext) -> Result<AppContext> {
    Ok(ctx
        .into_builder()
        .storage(Storage::single(storage::drivers::local::new()).into())
        .build())
}
```

### `SharedStore` — універсальний DI-слот

`shared_store: Arc<SharedStore>` (`src/app.rs:33-245, 272`) — це невелике гетерогенне сховище для сервісів, у яких немає власного поля в `AppContext`. Його API:

- `insert<T: 'static + Send + Sync>(&self, val: T)` (`:62`)
- `remove<T>(&self) -> Option<T>` (`:103`)
- `get_ref<T>(&self) -> Option<RefGuard<'_, T>>` (`:151`) — доступ за посиланням через guard з `Deref<Target = T>`
- `get<T: Clone + 'static + Send + Sync>(&self) -> Option<T>` (`:195`) — доступ через клонування
- `contains<T>(&self) -> bool` (`:221`)

Щоб прочитати збережене значення всередині обробника, використайте extractor з такою самою назвою: `controller::extractor::shared_store::SharedStore<T>(pub T)`, який реалізує `FromRequestParts<AppContext>` і повертає `Error::InternalServerError`, якщо `T` ніколи не було вставлено (`src/controller/extractor/shared_store.rs:6-29`).

**Примітка щодо назв:** `app::SharedStore` (контейнер, що зберігається в `AppContext`) та extractor `controller::extractor::shared_store::SharedStore<T>` (axum-extractor) — це два різні типи, які мають спільну назву. Обидва доступні через prelude — див. нижче, — тому розрізняйте їх за контекстом: тип поля — це store, а tuple-структура з generic-параметром — це extractor.

## `loco_rs::prelude`

`use loco_rs::prelude::*;` (`src/prelude.rs`) — це стандартний єдиний імпорт для коду застосунку (контролери, моделі, workers, задачі). Він реекспортує, безумовно, якщо не вказано інше:

**Async / інфраструктура axum**
- `async_trait::async_trait`
- `axum::debug_handler`
- `axum::extract::{Form, Multipart, Path, Query, State}`
- `axum::response::{IntoResponse, Response}`
- `axum::routing::{delete, get, head, options, patch, post, put, trace}`
- `axum_extra::extract::cookie`

**Сторонні помічники**
- `chrono::NaiveDateTime as DateTime`
- `include_dir::{include_dir, Dir}`
- `serde_json::json as data` — синтаксичний цукор, щоб код контролера/в'ю міг писати `data!({"item": ..})` замість `json!`
- `validator::Validate`

**Основні типи Loco**
- `app::{AppContext, Initializer}`
- `bgworker::{BackgroundWorker, Queue}`
- `errors::Error` та `Result` (аліас crate'а `Result<T, Error>`)
- `mailer` (сам модуль) та `mailer::Mailer`
- `task::{self, Task, TaskInfo}`
- `validation::{self, Validatable, ValidatorTrait}`

**Рівень контролерів**
- `controller::{bad_request, not_found, unauthorized}` — функції-конструктори відповідей з помилками
- `controller::format` — модуль побудови відповідей (`format::json`, `format::render()`, ...)
- `controller::middleware::format::{Format, RespondTo}` — extractor/enum узгодження вмісту
- `controller::middleware::remote_ip::RemoteIP` — extractor обчисленої IP-адреси клієнта
- `controller::middleware::MiddlewareStackExt` — помічники `insert_before` / `insert_after` / `replace` / `delete` для редагування стеку middleware за замовчуванням всередині `Hooks::middlewares` (див. [каталог middleware](/uk/docs/reference/middleware/)
- `controller::extractor::shared_store::SharedStore` — DI-extractor (див. вище)
- `controller::extractor::validate::{JsonValidate, JsonValidateWithMessage}` — extractors тіла запиту з валідацією
- `controller::views::{engines::TeraView, ViewEngine, ViewRenderer}`
- `controller::{Json, Routes}`

**Feature-залежні**
- `#[cfg(feature = "auth")] controller::extractor::auth` (весь модуль auth-extractors: `JWT`, `JWTWithUser`, `ApiToken`, `UserClaims`, помічники витягування токенів)
- `#[cfg(feature = "with-db")]`:
  - Трейти та типи Sea-ORM: `ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, Set, TransactionTrait`
  - Скалярні реекспорти Sea-ORM: `sea_orm::prelude::{Date, DateTimeUtc, DateTimeWithTimeZone, Decimal, Uuid}`
  - `model::{query, Authenticable, ModelError, ModelResult}` — плюс вкладений `pub mod model { pub use crate::model::query; }`, тому `query` доступний і як `loco_rs::prelude::query`, і як `loco_rs::prelude::model::query`
- `#[cfg(feature = "multi-tenancy")] model::{TenantEntity, TenantQueryExt, TenantActiveModelExt}` — опціональні помічники tenant-скоупінгу; ця feature також вмикає `with-db`
- `#[cfg(feature = "testing")] crate::testing::prelude::*` — підключається лише для застосунків/тестів, зібраних з feature `testing`

Джерело: `src/prelude.rs` (перевірено проти `HEAD` на момент написання).
