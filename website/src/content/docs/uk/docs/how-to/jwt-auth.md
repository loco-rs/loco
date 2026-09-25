---
title: Захищаємо маршрут за допомогою JWT
description: Додайте JWT-автентифікацію до маршруту за допомогою екстракторів JWT і JWTWithUser<T>, реалізуйте контракт Authenticable і генеруйте токени.
sidebar:
  order: 40
---

Мета: вимагати чинний JWT в обробнику та видавати токени, які ваші клієнти можуть надсилати у відповідь.

Loco постачає два екстрактори axum для маршрутів, захищених JWT:

- `auth::JWT` — валідує токен і дає вам claims. Працює без бази даних.
- `auth::JWTWithUser<T>` — валідує токен **і** завантажує запис користувача з бази даних через реалізацію [`Authenticable`](#контракт-authenticable) у вашій моделі. Потребує фічі `with-db`.

Обидва живуть у `loco_rs::controller::extractor::auth` і реекспортуються через `loco_rs::prelude::*`, щойно увімкнена фіча `auth`.

Якщо ви згенерували застосунок через `loco new` і обрали стартер із базою даних, `with-db` і `auth` уже увімкнені за замовчуванням — дивіться [довідник feature-прапорців](/uk/docs/reference/feature-flags/), якщо це треба перевірити чи змінити.

## 1. Увімкнюємо фічю `auth`

`auth` — це **типова** фіча (`Cargo.toml`): вона підтягує `jsonwebtoken` із його pure-Rust бекендом `rust_crypto`, тож для мінімальної збірки не потрібен жоден C-тулчейн. Якщо ви залежите від `loco-rs` з `default-features = false`, додайте її назад явно:

```toml
loco-rs = { version = "...", default-features = false, features = ["auth", "with-db"] }
```

## 2. Налаштовуємо секрет і строк дії

Додайте блок `auth.jwt` до `config/development.yaml` (і кожного іншого файла середовища):

```yaml
auth:
  jwt:
    secret: "<%= get_env(name='JWT_SECRET') %>" # обов'язково, має бути чинним base64
    expiration: 604800 # обов'язково, секунди (7 днів)
```

Два факти, які врятують вас від заплутаного повідомлення про помилку:

- **Секрет має бути чинним base64.** Loco кодує/декодує токени через `EncodingKey::from_base64_secret` / `DecodingKey::from_base64_secret`. Звичайний рядок, що не є base64, не падає під час завантаження конфігурації — він падає пізніше, коли токен генерується чи валідується, з помилкою, яка явно не вказує на конфігурацію. Згенеруйте base64-секрет, наприклад `openssl rand -base64 64`, і підставте його через `get_env`, як вище, замість хардкоду.
- **Типовий алгоритм підпису — HS512**, а не HS256. Він заданий у коді (`Algorithm::HS512`) і не є ключем YAML — перевизначайте його лише з Rust через `JWT::algorithm(..)` під час конструювання підписувача.

Повний довідник ключів `auth.jwt`, включно з `location`, — у [довіднику конфігурації](/uk/docs/reference/configuration/#auth). Розташуванню токена (`Bearer` / `Query` / `Cookie`) присвячений окремий гайд: [Налаштовуємо, де Loco шукає JWT](/uk/docs/how-to/jwt-locations/).

## 3. Генеруємо токен

Використайте `loco_rs::auth::jwt::JWT` (підписувач/валідатор — не плутайте з однойменним екстрактором нижче), щоб зкарбувати токен, зазвичай з обробника логіну, використовуючи секрет і строк дії, уже завантажені в `AppContext`:

```rust
use loco_rs::prelude::*;

async fn login(State(ctx): State<AppContext>, /* ... */) -> Result<Response> {
    // `user` — це те, що ви завантажили/перевірили під час логіну (див. hash-passwords.md).
    let jwt_config = ctx.config.get_jwt_config()?;

    let token = loco_rs::auth::jwt::JWT::new(&jwt_config.secret)
        .generate_token(jwt_config.expiration, user.pid.to_string(), serde_json::Map::new())
        .map_err(|e| Error::string(&e.to_string()))?;

    format::json(serde_json::json!({ "token": token }))
}
```

`generate_token` приймає строк дії (секунди), `pid`, який стає `claims.pid`, і необов'язкову `serde_json::Map` кастомних claims, що додаються поруч із `pid` у токені. Вона повертає результат `jsonwebtoken`, а не `Result` Loco, тож помилку треба відобразити явно, як показано.

## 4. Захищаємо маршрут: лише claims

Додайте `auth::JWT` як параметр обробника. Axum запускає його як екстрактор до виконання тіла вашого обробника; якщо токен відсутній, нерозбірливий, у неправильному місці або протермінований, запит ніколи не дістане вашого коду — Loco поверне за вас `401 Unauthorized`.

```rust
use loco_rs::prelude::*;
use loco_rs::controller::extractor::auth;

async fn current(
    auth: auth::JWT,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(serde_json::json!({ "pid": auth.claims.pid }))
}
```

`auth.claims` — це `UserClaims { pid, claims, .. }` — використовуйте `auth.claims.pid` для суб'єкта, а `auth.claims.claims` для будь-яких кастомних claims, які ви вписали при генерації. Цей екстрактор не потребує бази даних, тож працює навіть у застосунках без БД.

## 5. Захищаємо маршрут: claims + завантажений користувач

Коли обробнику потрібен фактичний рядок користувача (а не лише `pid`), використайте `auth::JWTWithUser<T>`, де `T` — ваша модель користувача. Це вимагає фічі `with-db` і вимагає, щоб `T` реалізував `Authenticable`.

```rust
use loco_rs::prelude::*;
use loco_rs::controller::extractor::auth;

async fn current(
    auth: auth::JWTWithUser<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(&auth.user)
}
```

Всередині `JWTWithUser` валідує токен точно так, як `auth::JWT`, а потім викликає `T::find_by_claims_key(&ctx.db, &claims.pid)`, щоб завантажити користувача. Відсутність запису в базі стає `401 Unauthorized`; помилка бази даних стає `500 Internal Server Error`.

## Контракт `Authenticable`

І `JWTWithUser<T>`, і `ApiToken<T>` (дивіться [гайд про API-ключі](/uk/docs/how-to/api-key-auth/) вимагають, щоб ваша модель користувача реалізувала `loco_rs::model::Authenticable`:

```rust
#[async_trait]
pub trait Authenticable: Clone {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self>;
    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self>;
}
```

Типова реалізація на Sea-ORM шукає рядок за відповідним стовпцем і відображає відсутність запису на `ModelError::EntityNotFound`. Трейт позначений `#[async_trait]`, тож ваш `impl` потребує того самого атрибута (`async_trait` реекспортується з `loco_rs::prelude`):

```rust
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
        let user = super::_entities::users::Entity::find()
            .filter(super::_entities::users::Column::Pid.eq(claims_key))
            .one(db)
            .await?;
        user.ok_or(ModelError::EntityNotFound)
    }
}
```

**Зауваження щодо охоплення:** трейт `Authenticable` і екстрактори вище визначає фреймворк; модель користувача, що їх реалізує, приходить із `loco new`, а не з `loco-gen`. Будь-який застосунок, згенерований із базою даних — тобто все, крім `--db none`, який обирає шаблон `lightweight-service`, — уже містить `src/models/users.rs` з реалізацією `Authenticable` та `src/controllers/auth.rs`, що монтує повний набір `/api/auth/*` (`register`, `verify/{token}`, `login`, `forgot`, `reset`, `current`, `magic-link`, `resend-verification-mail`). Прохід цим потоком дивіться в [туторіалі SaaS with auth](/uk/docs/tutorials/saas-with-auth/). З `--db none` моделі користувача взагалі немає, і наведений вище патерн — ваша стартова точка для підключення автентифікації до власної моделі.

## Перевіряємо, що працює

```sh
curl --location '127.0.0.1:5150/api/some/protected/route' \
     --header 'Authorization: Bearer <TOKEN>'
```

Відсутній, malformed або протермінований токен повертає `401 Unauthorized`. Чинний токен доходить до вашого обробника з заповненим `auth.claims` (і, для `JWTWithUser`, `auth.user`).

## Пов'язане

- [Налаштовуємо, де Loco шукає JWT](/uk/docs/how-to/jwt-locations/) — Bearer / Query / Cookie, одне місце чи кілька.
- [Захищаємо маршрут за допомогою API-ключа](/uk/docs/how-to/api-key-auth/) — `ApiToken<T>`, окремий механізм, що завжди читає Bearer-заголовок.
- [Хешуємо та перевіряємо паролі](/uk/docs/how-to/hash-passwords/) — для обробника логіну, що видає токен вище.
- [Довідник конфігурації](/uk/docs/reference/configuration/#auth) — усі ключі `auth.jwt`.
- [Довідник feature-прапорців](/uk/docs/reference/feature-flags/) — типові значення `auth` / `with-db` та їхні взаємодії.
- [Довідник AppContext і prelude](/uk/docs/reference/app-context/) — що `loco_rs::prelude::*` підтягує під фічєю `auth`.
```
