---
title: Запити DSL і пагінація
description: Струмкий фільтр-DSL ConditionBuilder, помічник діапазонів дат, API пагінації та типи помилок/Authenticable шару моделей.
sidebar:
  order: 10
---

`loco_rs::model::query` (досяжний як `loco_rs::prelude::query` або `loco_rs::prelude::model::query`) — це невеликий струмкий DSL над `Condition` з Sea-ORM, плюс помічники пагінації, що обгортають `PaginatorTrait` Sea-ORM. Модуль моделі також надає явне обмеження запитів Sea-ORM за тенантом. Ця сторінка документує повну поверхню операторів `ConditionBuilder`, тенантні трейти, `DateRangeBuilder`, `SortDirection`, типи/функції пагінації та типи помилок шару моделей `ModelError`/`ModelResult`/`Authenticable`, які повертає код запитів і моделей.

Модулі query та model вимагають ознаки `with-db`, яка вмикає `sea-orm`, `sea-orm-migration` і `sqlx`. Тенантні помічники додатково вимагають опціональної ознаки `multi-tenancy`, яка автоматично вмикає `with-db`.

## Обмеження за тенантом

З увімкненим `multi-tenancy` три трейти реекспортуються з `loco_rs::model` і прелюдії:

| Трейт | Реалізовується | Призначення |
|---|---|---|
| `TenantEntity` | Застосунком, для кожної сутності, що належить тенанту | Оголошує `type TenantId: Into<Value>` та `tenant_column() -> Self::Column`. |
| `TenantQueryExt` | Loco, для `Select<E>`, `UpdateMany<E>` і `DeleteMany<E>`, де `E: TenantEntity` | Додає `in_tenant(tenant_id)` — фільтр рівності на оголошеній тенантній колонці. |
| `TenantActiveModelExt` | Loco, для active-моделей, чия сутність реалізує `TenantEntity` | Додає `set_tenant(tenant_id) -> ModelResult<Self>`; встановлює порожній ключ, приймає той самий ключ і відхиляє перепризначення з `ModelError::TenantMismatch`. |

Обмеження є явним, а не збереженим у thread-local чи глобальному для запиту стані, тож його можна передавати між потоками виконавця і використовувати поза HTTP. Прямі конструктори Sea-ORM залишаються доступними для навмисних міжтенантних операцій. Див. [Додати багатоорендність на рівні рядків](/uk/docs/how-to/multi-tenancy/) щодо конфігурації ознаки, довіреного визначення тенанта та обмежених читань і записів.

## `ConditionBuilder` — струмкий фільтр-DSL

Модуль: `src/model/query/dsl/mod.rs`, реекспортований у `src/model/query/mod.rs:4`.

```rust
pub struct ConditionBuilder {
    condition: Condition, // sea_orm::Condition
}
```

Дві точки входу будують `ConditionBuilder`:

| Fn | Сигнатура | Поведінка | Anchor |
|---|---|---|---|
| `condition()` | `condition() -> ConditionBuilder` | Починає з `Condition::all()` (об'єднання через AND). | `dsl/mod.rs:38` |
| `with(condition)` | `const fn with(condition: Condition) -> ConditionBuilder` | Обгортає наявний `Condition` Sea-ORM (використовується всередині для ланцюжка викликів конструктора). | `dsl/mod.rs:45` |

`ConditionBuilder` реалізує `From<ConditionBuilder> for Condition` (`dsl/mod.rs:167`), і кожен метод-конструктор нижче повертає `Self` (споживаючи `self`), тож виклики ланцюжаться; `.build()` завершує до `sea_orm::Condition`:

```rust no-syntax-check="signature listing — no body, by design"
pub fn build(&self) -> Condition  // dsl/mod.rs:701
```

### Оператори

Кожен оператор існує двічі: як **вільна функція** `query::<op>(col, ..)`, що починає новий конструктор (скорочення для `condition().<op>(..)`), і як **метод** на `ConditionBuilder` для ланцюжка. Обидві форми приймають `ColumnTrait` Sea-ORM (згенерований перелік `Column` сутності) як аргумент-колонку.

| Оператор | Вільна fn (anchor) | Метод-конструктор (anchor) | Аргументи | SQL |
|---|---|---|---|---|
| Дорівнює | `eq` `dsl/mod.rs:51` | `eq` `:235` | `col: T, value: V: Into<Value>` | `col = value` |
| Не дорівнює | `not_equal` `:57` | `ne` `:260` | `col, value` | `col <> value` |
| Більше ніж | `gt` `:63` | `gt` `:285` | `col, value` | `col > value` |
| Більше або дорівнює | `gt_equal` `:69` | `gte` `:311` | `col, value` | `col >= value` |
| Менше ніж | `lt` `:75` | `lt` `:337` | `col, value` | `col < value` |
| Менше або дорівнює | `lt_equal` `:81` | `lte` `:363` | `col, value` | `col <= value` |
| Між | `between` `:87` | `between` `:389` | `col, a: V, b: V` | `col BETWEEN a AND b` |
| Поза межами | `not_between` `:93` | `not_between` `:415` | `col, a, b` | `col NOT BETWEEN a AND b` |
| Like | `like` `:99` | `like` `:441` | `col, pattern: V: Into<String>` | `col LIKE pattern` (wildcards подає викликач) |
| Not like | `not_like` `:105` | `not_like` `:467` | `col, pattern` | `col NOT LIKE pattern` |
| Починається з | `starts_with` `:111` | `starts_with` `:493` | `col, s: V: Into<String>` | `col LIKE 's%'` |
| Закінчується на | `ends_with` `:117` | `ends_with` `:519` | `col, s` | `col LIKE '%s'` |
| Містить | `contains` `:123` | `contains` `:545` | `col, s` | `col LIKE '%s%'` |
| Є null | `is_null` `:130` | `is_null` `:572` | `col` | `col IS NULL` |
| Не null | `is_not_null` `:137` | `is_not_null` `:599` | `col` | `col IS NOT NULL` |
| Є в | `is_in` `:144` | `is_in` `:626` | `col, values: I: IntoIterator<Item = V>` | `col IN (values...)` |
| Не в | `is_not_in` `:154` | `is_not_in` `:657` | `col, values` | `col NOT IN (values...)` |
| Діапазон дат | `date_range` `:163` | `date_range` `:696` | `col` — повертає `DateRangeBuilder<T>`, а не `Self` | див. [Діапазон дат](#daterangebuilder-фільтрація-за-діапазоном-дат) нижче |

Це 17 операторів порівняння/шаблонів/приналежності плюс `date_range`, що відповідає поверхні модуля приблизно з 18 операторів.

Приклад (з власного doctest модуля, `dsl/mod.rs:194-233`):

```rust
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, QueryFilter};

let cond = query::condition().eq(test_db::Column::Id, 1).build();
test_db::Entity::find().filter(cond);
// WHERE "loco"."id" = 1
```

`like`/`not_like` передають шаблон як є (wildcards `%` пише викликач); `starts_with`/`ends_with`/`contains` додають wildcard за вас і завжди компілюються в `LIKE`.

### `DateRangeBuilder` — фільтрація за діапазоном дат

`date_range(col)` (вільна fn або метод `ConditionBuilder`) повертає `DateRangeBuilder<T>` замість `Self`, бо діапазону дат потрібно 0, 1 або 2 межі, перш ніж він стане умовою. Структура та impl: `src/model/query/dsl/date_range.rs:7-66`.

| Метод | Сигнатура | Anchor |
|---|---|---|
| `new` | `const fn new(condition_builder: ConditionBuilder, col: T) -> Self` | `:15` |
| `dates` | `fn dates(self, from: Option<&NaiveDateTime>, to: Option<&NaiveDateTime>) -> Self` | `:25` |
| `from` | `fn from(self, from: &NaiveDateTime) -> Self` | `:35` |
| `to` | `fn to(self, to: &NaiveDateTime) -> Self` | `:45` |
| `build` | `fn build(self) -> ConditionBuilder` | `:54` |

**Поведінка меж несиметрична** (`date_range.rs:55-63`) — це єдина неочевидна семантика в DSL, яку варто знати до використання:

| Встановлені межі | SQL |
|---|---|
| ані `from`, ані `to` | умова не додається (passthrough) |
| лише `to` | `col < to` (строга) |
| лише `from` | `col > from` (строга) |
| і `from`, і `to` | `col BETWEEN from AND to` (включна) |

Тож односторонній діапазон ексклюзивний на межі, але двосторонній — включний на обох межах: `date_range(col).from(&d).build()` *не* включить рядки рівно на `d`, а `date_range(col).dates(Some(&d), Some(&d2)).build()` *включить* рядки рівно на `d` чи `d2`.

### `SortDirection`

`src/model/query/dsl/mod.rs:17-35`:

```rust
pub enum SortDirection {
    Desc, // serde "desc"
    Asc,  // serde "asc"
}
```

- Успадковує `Deserialize, Serialize` з `#[serde(rename = "desc"/"asc")]` на кожному варіанті — призначений для прямої десеріалізації з параметра сортування query-string.
- `order(&self) -> Order` (`:29`, `#[must_use] const fn`) перетворює на `sea_orm::sea_query::Order::Desc`/`Order::Asc` для використання з `.order_by(col, direction.order())`.

Цей перелік сам по собі не є оператором `ConditionBuilder` — він працює в парі з власним `QueryOrder::order_by` Sea-ORM, ортогонально до фільтрації.

## Пагінація

Модуль: `src/model/query/paginate/mod.rs`, реекспортований у `src/model/query/mod.rs:5`. Досяжний як `query::paginate`, `query::fetch_page`, `query::PaginationQuery`.

### `PaginationQuery`

```rust
pub struct PaginationQuery {
    pub page_size: u64, // за замовчуванням 25
    pub page: u64,      // за замовчуванням 1, нумерація з 1
}
```
(`paginate/mod.rs:31-45`)

| Поле | Тип | За замовчуванням | Примітки |
|---|---|---|---|
| `page_size` | `u64` | `25` (`default_page_size`, `:5-7`) | Рядків на сторінку. |
| `page` | `u64` | `1` (`default_page`, `:9-11`) | **Нумерація з 1.** `paginate`/`fetch_page` всередині роблять `saturating_sub(1)` для 0-базованого `fetch_page` Sea-ORM. |

- Обидва поля використовують власний `deserialize_pagination_filter` (`:69-75`), який парсить **рядок** у `u64` — обхідний маневр навколо баги `serde_urlencoded`, коли числові параметри query-string не десеріалізуються напряму в цілі числа. Це робить `PaginationQuery` безпечним для використання як поля `#[serde(flatten)]` всередині структури екстрактора axum `Query<T>`, наприклад:

  ```rust
  #[derive(Debug, Deserialize)]
  pub struct ListQueryParams {
      pub title: Option<String>,
      pub content: Option<String>,
      #[serde(flatten)]
      pub pagination: query::PaginationQuery,
  }
  ```
  (doctest у `paginate/mod.rs:19-30`)

- `PaginationQuery::page(page: u64) -> Self` (`:49`) — конструює з даною сторінкою та `page_size` за замовчуванням через `..Default::default()`.
- `impl Default for PaginationQuery` (`:58-65`) — `page_size = 25`, `page = 1`.

### `PageResponse<T>` і `PagerMeta`

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct PageResponse<T> {
    pub page: Vec<T>,
    pub meta: PagerMeta,
}
```
(`paginate/mod.rs:80-83`; `PagerMeta` — це `crate::controller::views::pagination::PagerMeta`, імпортований у `:77`)

`PagerMeta` (`src/controller/views/pagination.rs:13-22`) — не перестворений інвентарем, перевірений безпосередньо з вихідного коду для цієї сторінки:

```rust
pub struct PagerMeta {
    pub page: u64,       // серіалізується як "page"
    pub page_size: u64,  // серіалізується як "page_size"
    pub total_pages: u64,// серіалізується як "total_pages"
    pub total_items: u64,// серіалізується як "total_items"
}
```

### `paginate` і `fetch_page`

| Fn | Сигнатура | Anchor |
|---|---|---|
| `paginate` | `async fn paginate<E>(db: &DatabaseConnection, entity: Select<E>, condition: Option<Condition>, pagination_query: &PaginationQuery) -> LocoResult<PageResponse<E::Model>> where E: EntityTrait, E::Model: Sync` | `paginate/mod.rs:146` |
| `fetch_page` | `async fn fetch_page<'db, C, S>(db: &'db C, selector: S, pagination_query: &PaginationQuery) -> LocoResult<PageResponse<...>> where C: ConnectionTrait + Sync, S: PaginatorTrait<'db, C> + Send` | `paginate/mod.rs:204` |

Обидві:
- Беруть `pagination_query.page` викликача (нумерація з 1) і всередині роблять `.saturating_sub(1)` (`:156`, `:213`) перед викликом `Paginator::fetch_page` Sea-ORM (який 0-базований).
- Викликають `query.num_items_and_pages().await?` для заповнення `PagerMeta.total_pages`/`total_items`, потім `query.fetch_page(page).await?` для даних рядків.
- Повертають `Ok(PageResponse { page, meta })` — крейтовий `LocoResult<T>` (тобто `crate::Result<T, crate::errors::Error>`).

`paginate` приймає `Select<E>` (конструктор запиту сутності) плюс опціональну попередньо побудовану `Condition` — він сам застосовує `.filter(condition)`, якщо умова дана, тож вам не треба ланцюжити `.filter()` перед викликом. `fetch_page` — більш загальна форма: вона приймає все, що реалізує `PaginatorTrait` Sea-ORM напряму (тож можна попередньо побудувати довільні select-и, включно з `.order_by(..)`, і просто пагінувати результат), але не приймає окремого аргументу `Condition` — фільтр і сортування мусять бути вже застосовані до селектора, який ви передаєте.

```rust
// paginate: сутність + опціональна умова + запит пагінації
let condition = query::condition().contains(Column::Name, "loco").build();
let res = query::paginate(&db, Entity::find(), Some(condition), &pagination_query).await;

// fetch_page: попередньо побудований селектор (будь-який PaginatorTrait), без окремого аргументу умови
let res = query::fetch_page(&db, Entity::find(), &query::PaginationQuery::page(2)).await;
```
(адаптовано з doctests у `paginate/mod.rs:92-140` та `:185-197`)

## Типи помилок шару моделей

`src/model/mod.rs` — тип помилки, який повертає код моделей/автентифікації (окремий від крейтового `loco_rs::errors::Error`; див. [довідник моделі помилок](/uk/docs/reference/errors/).

### `ModelError` / `ModelResult`

```rust
pub enum ModelError {
    EntityAlreadyExists,
    EntityNotFound,
    #[cfg(feature = "multi-tenancy")]
    TenantMismatch,
    Validation(ModelValidationErrors),      // #[from]
    #[cfg(feature = "auth")]
    Jwt(jsonwebtoken::errors::Error),        // #[from]
    DbErr(sea_orm::DbErr),                   // #[from]
    Any(Box<dyn std::error::Error + Send + Sync>), // #[from]
    Message(String),
}

pub type ModelResult<T, E = ModelError> = std::result::Result<T, E>;
```
(`src/model/mod.rs:13-35` для переліку, `:38` для аліасу)

`ModelError` є `#[non_exhaustive]`, як і крейтовий `Error`, тож downstream-зіставлення вимагають wildcard-arms. З увімкненим `multi-tenancy` `TenantMismatch` повертається, коли `TenantActiveModelExt::set_tenant` мав би перезаписати ключ іншого тенанта. `Jwt` існує лише з увімкненою ознакою `auth`.

Конструктори:

| Fn | Сигнатура | Anchor |
|---|---|---|
| `ModelError::wrap` | `#[must_use] fn wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self` — будує `Any(Box::new(err))` | `:42-44` |
| `ModelError::to_msg` | `#[must_use] fn to_msg(err: impl std::error::Error + Send + Sync + 'static) -> Self` — будує `Message(err.to_string())` | `:47-49` |
| `ModelError::msg` | `#[must_use] fn msg(s: &str) -> Self` — будує `Message(s.to_string())` | `:52-54` |

Сам `loco_rs::errors::Error` має варіант `Model(#[from] crate::model::ModelError)` (обмежений `with-db`), тож `ModelError`, повернений з методу моделі, автоматично перетворюється на крейтовий `Error` на межі контролера через `?`.

### `Authenticable`

```rust
#[async_trait]
pub trait Authenticable: Clone {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self>;
    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self>;
}
```
(`src/model/mod.rs:56-60`)

Модель користувача (зазвичай сутність `users`) реалізує `Authenticable`, щоб auth-екстрактори (`JWT`, `JWTWithUser`, `ApiToken` під `prelude::auth`, ознака `auth`) могли знайти викликача: `find_by_claims_key` розв'язує subject claims JWT до екземпляра моделі; `find_by_api_key` так само розв'язує bearer/API-key-заголовок. Обидва — `async` (сам трейт `#[async_trait]`) і повертають `ModelResult<Self>`, тож невдалий пошук виявляється як `ModelError` (зазвичай `EntityNotFound` чи `DbErr`).

Модуль запитів, помилки моделей і `Authenticable` реекспортуються з `loco_rs::prelude` за ознакою `with-db`. Три тенантні трейти реекспортуються за ознакою `multi-tenancy`.
