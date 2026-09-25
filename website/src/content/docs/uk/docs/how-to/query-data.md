---
title: Запит даних за допомогою condition DSL
description: Фільтруйте сутності операторами ConditionBuilder, формуйте діапазони дат і сортуйте результати — без ручного складання умов Sea-ORM.
sidebar:
  order: 2
---

**Мета:** фільтрувати рядки з моделі за допомогою плавного DSL `ConditionBuilder` від Loco замість ручного збирання `Condition` у Sea-ORM.

Це передбачає робочу модель (дивіться [Додати модель](/uk/docs/how-to/add-model/). Повний перелік операторів, точні сигнатури та семантику меж `date_range` дивіться у [Query DSL & pagination](/uk/docs/reference/query-pagination/).

## 1. Імпортуйте DSL

`query` — це модуль запитів на рівні моделі, доступний прямо з прелюдії:

```rust
use loco_rs::prelude::*;
use sea_orm::EntityTrait;
```

`query::condition()` запускає білдер; `.build()` завершує його, перетворюючи на `sea_orm::Condition`, який ви передаєте до `.filter(..)`.

Для даних, що належать орендарям, увімкніть можливість `multi-tenancy`, оголосіть `TenantEntity` та додавайте `.in_tenant(tenant_id)` до вибірок і групових мутацій. Дивіться [Мультиорендарність на рівні рядків](/uk/docs/how-to/multi-tenancy/).

## 2. Побудуйте простий фільтр

```rust
let cond = query::condition()
    .eq(users::Column::Email, "user1@example.com")
    .build();

let user = users::Entity::find().filter(cond).one(&db).await?;
```

Це той самий шаблон, який модель `users` демо-застосунку використовує для всіх своїх пошуків, наприклад `Model::find_by_email` у `examples/demo/src/models/users.rs`:

```rust
pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
    let user = users::Entity::find()
        .filter(
            model::query::condition()
                .eq(users::Column::Email, email)
                .build(),
        )
        .one(db)
        .await?;
    user.ok_or_else(|| ModelError::EntityNotFound)
}
```

## 3. Ланцюжково поєднуйте кілька умов

Кожен оператор повертає `Self`, тож умови зчеплюються та за замовчуванням поєднуються через AND:

```rust
let cond = query::condition()
    .contains(posts::Column::Title, "loco")
    .gt(posts::Column::Views, 10)
    .is_not_null(posts::Column::PublishedAt)
    .build();

let published = posts::Entity::find().filter(cond).all(&db).await?;
```

Доступні оператори (повна таблиця — у [довіднику](/uk/docs/reference/query-pagination/#оператори): `eq`/`ne`, `gt`/`gte`/`lt`/`lte`, `between`/`not_between`, `like`/`not_like`, `starts_with`/`ends_with`/`contains`, `is_null`/`is_not_null`, `is_in`/`is_not_in`, а також `date_range`. Кожен з них існує також як вільна функція (`query::eq(col, v)`), що запускає новий білдер — зручно, коли потрібна лише одна умова.

## 4. Фільтруйте за діапазоном дат

`date_range` повертає `DateRangeBuilder` замість `Self`, оскільки діапазон може мати нуль, одну або дві межі:

```rust no-syntax-check="`/* ... */` стоїть замість значень, які читач підставляє сам"
use chrono::NaiveDateTime;

let from: NaiveDateTime = /* ... */;
let to: NaiveDateTime = /* ... */;

let cond = query::condition()
    .date_range(posts::Column::CreatedAt)
    .dates(Some(&from), Some(&to))
    .build() // DateRangeBuilder -> ConditionBuilder
    .build(); // ConditionBuilder -> Condition
```

<div class="infobox">
Поведінка меж асиметрична: односторонній діапазон (лише <code>from</code> або лише <code>to</code>) є <b>строгим</b> (<code>&gt;</code> / <code>&lt;</code>), а двосторонній діапазон (і <code>from</code>, і <code>to</code>) — <b>включним</b> (<code>BETWEEN</code>). Повна таблиця — у <a href="/uk/docs/reference/query-pagination/#daterangebuilder--фільтрація-за-діапазоном-дат">довіднику</a>.
</div>

## 5. Сортуйте результати

`SortDirection` — це невеликий enum, дружній до serde, який перетворюється на `Order` з Sea-ORM — він ортогональний до `ConditionBuilder` і задуманий для пари з `.order_by()`:

```rust
use loco_rs::model::query::SortDirection;

let direction = SortDirection::Desc;

let recent = posts::Entity::find()
    .filter(cond)
    .order_by(posts::Column::CreatedAt, direction.order())
    .all(&db)
    .await?;
```

Оскільки `SortDirection` виводить `Deserialize`/`Serialize` з перейменуваннями `"asc"`/`"desc"`, він десеріалізується безпосередньо з параметра рядка запиту (наприклад, `?sort=desc` в екстракторі `Query<T>` контролера).

## Результат

Ви отримали `Condition`, побудований з читабельних методів, що зчеплюються в ланцюжок, замість шаблонного коду на кшталт `Condition::all().add(...)` у чистому Sea-ORM, і він компонуеться з `.filter()`, `.order_by()` та — як покаже наступний гайд — з `query::paginate`.

## Далі

- [Розбивка результатів на сторінки](/uk/docs/how-to/paginate/) з використанням щойно побудованої умови.
- [Довідник Query DSL & pagination](/uk/docs/reference/query-pagination/) — точний SQL-вивід кожного оператора.
