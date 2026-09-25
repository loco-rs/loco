---
title: Розбивка результатів запиту на сторінки
description: Горніть сутності сторінками через paginate/fetch_page, приймайте page/page_size із запиту через PaginationQuery та повертайте PageResponse з контролера.
sidebar:
  order: 3
---

**Мета:** повертати з ендпоінта контролера сторінку рядків разом із метаданими пагінації (`total_pages`, `total_items`, ...) замість завантаження цілої таблиці.

Це продовження [Запит даних за допомогою condition DSL](/uk/docs/how-to/query-data/). Точні сигнатури та структуру `PagerMeta` дивіться у [Query DSL & pagination](/uk/docs/reference/query-pagination/#пагінація).

## 1. Приймайте параметри пагінації у запиті

`PaginationQuery` — це невелика структура, дружня до `#[serde(flatten)]`, з двома полями: `page` (починаючи з 1, типово `1`) та `page_size` (типово `25`). Розгорніть її у власній структурі параметрів запиту контролера, щоб клієнти могли передавати `?page=2&page_size=10` разом з вашими власними фільтрами:

```rust
use loco_rs::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListQueryParams {
    pub title: Option<String>,
    #[serde(flatten)]
    pub pagination: query::PaginationQuery,
}
```

## 2. Розбийте запит до сутності на сторінки з опційною умовою

`query::paginate` приймає `Select<E>` (нерозв'язаний запит до сутності), опційну наперед побудовану `Condition` та `&PaginationQuery`. Він застосовує умову за вас, тож не викликайте `.filter()` самостійно перед цим:

```rust
use axum::extract::{Query, State};

pub async fn list(
    State(ctx): State<AppContext>,
    Query(params): Query<ListQueryParams>,
) -> Result<Response> {
    let condition = params
        .title
        .as_ref()
        .map(|t| query::condition().contains(posts::Column::Title, t).build());

    let res = query::paginate(
        &ctx.db,
        posts::Entity::find(),
        condition,
        &params.pagination,
    )
    .await?;

    format::json(res)
}
```

## 3. Або розбийте наперед побудований селектор через `fetch_page`

Використовуйте `fetch_page`, коли ви вже склали `Select` (з власними `.filter()`/`.order_by()`/join-ами) і вам потрібно лише пройтися по ньому сторінками — він не приймає окремого аргумента-умови:

```rust
let selector = posts::Entity::find()
    .filter(query::condition().eq(posts::Column::UserId, user_id).build())
    .order_by_desc(posts::Column::CreatedAt);

let res = query::fetch_page(&ctx.db, selector, &query::PaginationQuery::page(2)).await?;
```

## 4. Що ви отримуєте у відповідь

Обидві функції повертають `LocoResult<PageResponse<T>>`:

```rust
pub struct PageResponse<T> {
    pub page: Vec<T>,
    pub meta: PagerMeta,
}

pub struct PagerMeta {
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
    pub total_items: u64,
}
```

Повернення `res` з контролера (наприклад, через `format::json(res)`) серіалізується так:

```json
{
  "page": [ { "id": 1, "title": "..." }, ... ],
  "meta": { "page": 2, "page_size": 25, "total_pages": 4, "total_items": 87 }
}
```

## 5. Типізований конверт зі скаффолда

Згенерований обробник `list` відповідає через `Page<T>` (`src/dtos/common.rs`), а не безпосередньо `PageResponse`, оскільки типізованому React-фронтенду потрібен тип, експортований через `ts-rs`, а пласке тіло споживати простіше:

```json
{
  "items": [ { "id": 1, "title": "..." } ],
  "page": 2,
  "page_size": 25,
  "total_pages": 4,
  "total_items": 87
}
```

Назви полів метаданих — це поля `PagerMeta`, тож у застосунку існує єдина лексика пагінації, який би конверт обробник не повертав. Створюйте його з результату пагінованого запиту, а не вручну — саме це утримує їх узгодженими:

```rust
let res = query::paginate(
    &ctx.db,
    posts::Entity::find().order_by_asc(posts::Column::Id),
    None,
    &pagination,
)
.await?;

Ok(Json(Page::from_query(res)))
```

`Page::from_query` відображає кожну модель через `T: From<M>` — той `impl From<Model> for PostDto`, який скаффолд вже генерує.

## Результат

Запит на кшталт `GET /posts?page=2&page_size=10&title=loco` повертає рівно одну сторінку відповідних рядків плюс достатньо метаданих, щоб клієнт міг відрендерити «сторінка 2 з 4» або побудувати посилання next/prev — без завантаження цілої таблиці та ручної математики з `OFFSET`/`LIMIT`. Пам'ятайте: `page` на вході починається з 1; обидві функції внутрішньо виконують перетворення на 0-нульову пагінацію Sea-ORM.

## Далі

- [Довідник Query DSL & pagination](/uk/docs/reference/query-pagination/) — точні типові значення `PaginationQuery` та сигнатури `paginate`/`fetch_page`.
- [Запит даних](/uk/docs/how-to/query-data/) — побудова `Condition`, який ви передаєте.
