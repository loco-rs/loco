---
title: Захищаємо маршрут за допомогою API-ключа
description: "Автентифікуйте запити персональним API-ключем користувача за допомогою екстрактора ApiToken<T> та Authenticable::find_by_api_key."
sidebar:
  order: 41
---

Мета: автентифікувати запит довгоживучим персональним API-ключем користувача замість JWT — корисно для machine-to-machine або CLI-клієнтів, яким не довелося б повторно автентифікуватися за свіжим токеном.

Loco надає `auth::ApiToken<T>` — екстрактор axum, який зчитує ключ із заголовка `Authorization` і завантажує відповідного користувача через `Authenticable::find_by_api_key` вашої моделі.

## Передумови

- Фіча `with-db` (увімкнена за замовчуванням) — `ApiToken<T>` компілюється лише під `#[cfg(feature = "with-db")]`.
- Ваша модель користувача реалізує `loco_rs::model::Authenticable`, зокрема `find_by_api_key`. Форму трейта та приклад реалізації дивіться в [контракті `Authenticable`](/uk/docs/how-to/jwt-auth/#контракт-authenticable).
- Стовпець у моделі користувача для зберігання ключа (наприклад, `api_key`) і спосіб його заповнення — `loco_rs::hash::random_string` — зручний генератор; дивіться [Хешуємо та перевіряємо паролі](/uk/docs/how-to/hash-passwords/#генеруємо-випадкові-токени).

На відміну від JWT-автентифікації, `ApiToken<T>` не потребує **жодної конфігурації `auth.jwt`** — він не звертається до `auth.jwt.secret`/`expiration`/`location`. Йому потрібні лише база даних і реалізація `Authenticable`.

## 1. Реалізуємо `find_by_api_key`

```rust
use async_trait::async_trait;
use loco_rs::model::{Authenticable, ModelError, ModelResult};
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter};

#[async_trait]
impl Authenticable for super::_entities::users::Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = super::_entities::users::Entity::find()
            .filter(super::_entities::users::Column::ApiKey.eq(api_key))
            .one(db)
            .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        // Потрібен трейтом, навіть якщо цей застосунок використовує лише ApiToken.
        // Дивіться jwt-auth.md, якщо ви також підтримуєте JWTWithUser<T>.
        unimplemented!()
    }
}
```

## 2. Додаємо `ApiToken<T>` в обробник

```rust
use loco_rs::prelude::*;
use loco_rs::controller::extractor::auth;

async fn current_by_api_key(
    auth: auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(&auth.user)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("user")
        .add("/current-api", get(current_by_api_key))
}
```

`auth.user` — це повністю завантажений `T` (ваша модель `Authenticable`) — на відміну від JWT-екстракторів, тут немає окремої структури claims, яку треба розгортати.

## 3. Надсилаємо ключ як Bearer-токен

**`ApiToken<T>` завжди зчитує ключ із заголовка `Authorization: Bearer <key>`, і лише звідти.** Це жорстко зашите зчитування (`extract_token_from_header`), незалежне від будь-якого налаштування `auth.jwt.location` — конфігурація `location` (`Bearer`/`Query`/`Cookie`, описана в [Налаштовуємо, де Loco шукає JWT](/uk/docs/how-to/jwt-locations/) стосується лише екстракторів `JWT`/`JWTWithUser`, і ніколи — `ApiToken`.

```sh
curl --location '127.0.0.1:5150/api/user/current-api' \
     --header 'Authorization: Bearer <API_KEY>'
```

## Перевіряємо, що працює

- Чинний, відомий ключ повертає користувача (200, з JSON-тілом вашого обробника).
- Невідомий ключ повертає `401 Unauthorized` (пошук у моделі не знаходить запис, відображений із `ModelError::EntityNotFound`).
- Помилка бази даних під час пошуку ключа повертає `500 Internal Server Error` і логується через `tracing::error!`.

## Пов'язане

- [Захищаємо маршрут за допомогою JWT](/uk/docs/how-to/jwt-auth/) — контракт `Authenticable` повністю, а також екстрактори `JWT` / `JWTWithUser<T>`.
- [Налаштовуємо, де Loco шукає JWT](/uk/docs/how-to/jwt-locations/) — стосується JWT-автентифікації, а не `ApiToken`.
- [Хешуємо та перевіряємо паролі](/uk/docs/how-to/hash-passwords/) — генерація випадкового значення ключа для зберігання в кожного користувача.
- [Довідник feature-прапорців](/uk/docs/reference/feature-flags/) — типове значення `with-db` і те, що воно керує.
