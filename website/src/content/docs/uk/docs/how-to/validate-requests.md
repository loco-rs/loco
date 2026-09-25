---
title: Валідація запитів
description: Валідуйте JSON, форми та рядки запиту за допомогою валідувальних екстракторів Loco і повертайте структуровані або спрощені помилки.
sidebar:
  order: 11
---

**Мета:** відхиляти некоректні вхідні дані до того, як виконається логіка обробника, і повертати або простий `400 Bad Request`, або структуроване по-поле-за-полем JSON-тіло помилки.

Це передбачає наявність робочого контролера — дивіться [Додаємо контролер](/uk/docs/how-to/add-controller/), якщо його ще немає. Усі шість екстракторів живуть у `loco_rs::controller::extractor::validate` (`JsonValidate`/`JsonValidateWithMessage` реекспортовані з `loco_rs::prelude`).

## 1. Виберіть екстрактор

| Екстрактор | Content type | Структуровані JSON-помилки? |
|---|---|---|
| `JsonValidate<T>` | `application/json` | Ні — простий `400 Bad Request` |
| `JsonValidateWithMessage<T>` | `application/json` | Так |
| `FormValidate<T>` | `application/x-www-form-urlencoded` | Ні — простий `400 Bad Request` |
| `FormValidateWithMessage<T>` | `application/x-www-form-urlencoded` | Так |
| `QueryValidate<T>` | будь-який (читає рядок запиту) | Ні — простий `400 Bad Request` |
| `QueryValidateWithMessage<T>` | будь-який (читає рядок запиту) | Так |

Кожен із них — це newtype з `FromRequest`: `JsonValidate<T>(pub T)` тощо — розпакуйте через pattern matching, щоб дістати внутрішній, уже валідований `T`.

## 2. Визначте тип, що валідується

Виведіть (derive) `validator::Validate`:

```rust
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateNote {
    #[validate(length(min = 3, message = "title must be at least 3 characters"))]
    pub title: String,
    #[validate(email)]
    pub email: String,
}
```

`validator::Validate` автоматично адаптується до власного `ValidatorTrait` Loco — вам не потрібно реалізовувати нічого додаткового, щоб використовувати його з екстракторами вище.

## 3. Використайте екстрактор в обробнику

```rust
use loco_rs::prelude::*;

#[debug_handler]
pub async fn create(
    State(_ctx): State<AppContext>,
    JsonValidate(params): JsonValidate<CreateNote>,
) -> Result<Response> {
    // `params` гарантовано валідний тут
    format::json(params)
}
```

Змініть тип екстрактора, щоб змінити джерело/поведінку — тіло обробника в іншому не змінюється:

```rust
#[debug_handler]
pub async fn search(
    QueryValidateWithMessage(params): QueryValidateWithMessage<CreateNote>,
) -> Result<Response> {
    format::json(params)
}
```

`QueryValidate`/`QueryValidateWithMessage` читають рядок запиту URL (наприклад, `?title=abc&email=a@b.com`) незалежно від `Content-Type` запиту.

## 4. Що бачить клієнт у разі невдачі

`JsonValidate`/`FormValidate`/`QueryValidate` (без `WithMessage`) повертають «голий» `400`:

```json
{ "error": "Bad Request" }
```

Варіанти `*WithMessage` повертають по-поле-за-полем деталі під ключем `errors`, **без** заповнених `error`/`description`:

```json
{
  "errors": {
    "title": [
      { "code": "length", "message": "title must be at least 3 characters", "params": { "min": 3, "value": "ab" } }
    ],
    "email": [
      { "code": "email", "message": null, "params": { "value": "not-an-email" } }
    ]
  }
}
```

Це та сама гілка `Validation` у [мапі помилка → HTTP-статус](/uk/docs/reference/errors/) (завжди `400`) — екстрактори `WithMessage` заповнюють `errors`, а звичайні маплять невдалу валідацію на `Error::BadRequest` без повідомлення.

Некоректні вхідні дані, які екстрактор узагалі не може десеріалізувати (поганий JSON, непарсовний рядок запиту), також повертають `400`, ще до запуску валідації.

## 5. Валідація без крейту `validator`

Реалізуйте `ValidatorTrait` безпосередньо для повного контролю — корисно, коли правило не вписується у derive-макроси `validator`:

```rust
use loco_rs::prelude::*;
// prelude реекспортує лише `validation::{self, Validatable, ValidatorTrait}`,
// тому оголосіть два типи помилок явно
use loco_rs::validation::{ModelValidationErrors, ValidationError};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, serde::Deserialize)]
pub struct CustomParams {
    pub name: String,
}

impl ValidatorTrait for CustomParams {
    fn validate(&self) -> Result<(), ModelValidationErrors> {
        if self.name.len() < 5 {
            let mut errors: BTreeMap<String, Vec<ValidationError>> = BTreeMap::new();
            let mut params: HashMap<String, serde_json::Value> = HashMap::new();
            params.insert("min".to_string(), serde_json::json!(5));
            errors.insert(
                "name".to_string(),
                vec![ValidationError { code: "length".to_string(), message: None, params }],
            );
            return Err(ModelValidationErrors { errors });
        }
        Ok(())
    }
}
```

`ValidationError` має три поля: `code: String`, `message: Option<String>`, `params: HashMap<String, serde_json::Value>`. Порожні `params` автоматично пропускаються в JSON. Будь-який тип, що реалізує `ValidatorTrait`, працює з усіма шістьма екстракторами вище — екстрактору байдуже, звідки прийшла валідація: з крейту `validator` чи з вашої власної реалізації.

## Перевірка

```sh
curl -s -X POST localhost:5150/api/notes -H 'content-type: application/json' -d '{"title":"ab","email":"bad"}'
# {"errors":{"title":[...],"email":[...]}}   (JsonValidateWithMessage)
# {"error":"Bad Request"}                    (JsonValidate)
```

## Далі

- [Обробка помилок](/uk/docs/how-to/handle-errors/) — повна мапа помилка → статус і `CustomError`
- [Відповіді в різних форматах](/uk/docs/how-to/respond-formats/)
