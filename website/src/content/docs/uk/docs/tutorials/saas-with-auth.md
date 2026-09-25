---
title: Створіть невеликий застосунок з автентифікацією
description: Згенеруйте застосунок зі стартового шляху SaaS, зареєструйте користувача та увійдіть, викличте ендпоінт, захищений JWT, а потім захистіть свій власний.
sidebar:
  order: 3
---

Кожен застосунок Loco, згенерований з базою даних, постачається з повним набором автентифікації: реєстрація, вхід, підтвердження електронної пошти, скидання пароля, магічні посилання та ендпоінт «поточний користувач», захищений JWT — без жодного додаткового генератора, без пошуку стартового шаблону. Цей урок генерує такий застосунок, перевіряє вбудований потік автентифікації від початку до кінця, а потім захищає ваш власний ресурс тим самим JWT-екстрактором, який використовують вбудовані ендпоінти.

Ви вже маєте бути знайомі з основами з [Вашого першого застосунку](/uk/docs/tutorials/your-first-app/).

## 1. Згенеруйте застосунок

```sh
loco new --name saas_app --db sqlite --bg async --assets serverside
cd saas_app
```

Це та сама комбінація прапорців, з якою згенеровано власний застосунок `examples/demo` у репозиторії loco-rs — серверні асети, SQLite та асинхронні воркери в процесі. Саме вибір бази даних вмикає автентифікацію: скафолди `auth` та `mailer` включаються автоматично, коли `--db` є `sqlite` або `postgres`, незалежно від того, який стартер ви обрали інтерактивно — окремого прапорця «SaaS» запам'ятовувати не потрібно.

Перевірте, що маршрути автентифікації вже на місці, ще до того, як щось згенеровано:

`routes` друкує дерево сегментів шляхів, де праворуч вказано метод кожного ендпоінта та його повний шлях:

```sh
$ cargo loco routes
/_health GET                              /_health
/_ping GET                                /_ping
/_readiness GET                           /_readiness
/api
   └─ /auth
      ├─ /current GET                     /api/auth/current
      ├─ /forgot POST                     /api/auth/forgot
      ├─ /login POST                      /api/auth/login
      ├─ /magic-link
      │  ├─ POST                          /api/auth/magic-link
      │  └─ /{token} GET                  /api/auth/magic-link/{token}
      ├─ /register POST                   /api/auth/register
      ├─ /resend-verification-mail POST   /api/auth/resend-verification-mail
      ├─ /reset POST                      /api/auth/reset
      └─ /verify/{token} GET              /api/auth/verify/{token}
```

Три маршрути з префіксом `_` — це вбудовані ендпоінти моніторингу Loco, додані `AppRoutes::with_default_routes()`; усе під `/api/auth` прийшло разом із базою даних.

## 2. Уникніть залежності від SMTP

Реєстрація користувача надсилає вітальний лист через налаштований поштовий сервіс. `config/development.yaml` за замовчуванням вмикає SMTP на `localhost:1025`, що означає: реєстрація завершиться помилкою 500, якщо ви або не запустите локальний SMTP-уловлювач там, або не вкажете поштовому сервісу заглушувати вихідні листи замість їх надсилання. Для цього уроку заглуште його — відкрийте `config/development.yaml` і додайте `stub: true` під `mailer`:

```yaml
mailer:
  stub: true
  smtp:
    enable: true
    host: localhost
    # ...
```

<div class="infobox">
Згенерований <code>config/test.yaml</code> уже встановлює <code>mailer.stub: true</code> — саме тому тестам вашого застосунку ніколи не потрібна жива поштова скринька. Ви застосовуєте те саме налаштування до <code>development.yaml</code>, щоб <code>cargo loco start</code> поводився так само.
</div>

## 3. Запустіть застосунок

```sh
cargo loco start
```

## 4. Зареєструйте користувача

```sh
$ curl --location 'localhost:5150/api/auth/register' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "name": "Loco user",
         "email": "user@loco.rs",
         "password": "12341234"
     }'
{}
```

Порожній `{}` при успіху є навмисним: ендпоінт завжди відповідає однаково, незалежно від того, чи була електронна пошта вже зареєстрована, тому жоден запит не можна використати для зондування списку ваших користувачів.

## 5. Увійдіть

```sh
$ curl --location 'localhost:5150/api/auth/login' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

```json
{
  "token": "eyJhbGciOiJIUzUxMiJ9...",
  "pid": "2b20f998-b11e-4aeb-96d7-beca7671abda",
  "name": "Loco user",
  "is_verified": false
}
```

`is_verified` дорівнює `false`, оскільки ви не натиснули (заглушений, ненадісланий) лист для підтвердження — це нормально, **вхід не вимагає підтвердженої електронної пошти**, лише збігу пароля. Збережіть `token`; кожен автентифікований запит нижче використовує його як bearer-токен.

## 6. Викличте вбудований захищений ендпоінт

```sh
$ curl --location 'localhost:5150/api/auth/current' \
     --header 'Authorization: Bearer TOKEN'
```

```json
{
  "pid": "2b20f998-b11e-4aeb-96d7-beca7671abda",
  "name": "Loco user",
  "email": "user@loco.rs"
}
```

Під капотом `current` — нічого особливого: це звичайний обробник, який приймає `auth::JWT` як перший аргумент:

```rust
async fn current(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(CurrentResponse::new(&user))
}
```

Якщо заголовок `Authorization` відсутній, некоректний або містить прострочений/недійсний токен, axum ніколи не дійде до тіла вашого обробника — сам екстрактор `auth::JWT` відхилить запит із `401 Unauthorized`. Спробуйте без заголовка, щоб побачити це на власні очі.

## 7. Захистіть власний ресурс

Наведений вище шаблон працює для будь-якого обробника, а не лише для вбудованих. Згенеруйте скафолд `notes`:

```sh
$ cargo loco generate scaffold notes title:string content:text
```

Відкрийте `src/controllers/notes.rs` і змініть сигнатуру обробника `add`, щоб він також вимагав `auth::JWT`:

```rust
pub async fn add(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<Params>,
) -> Result<Response> {
    // we only need to know the request carries a valid, known user
    let _current_user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    let mut item = ActiveModel { ..Default::default() };
    params.update(&mut item);
    let item = item.insert(&ctx.db).await?;
    format::json(item)
}
```

`auth::JWT` уже є в області видимості через `loco_rs::prelude::*`, який імпортує кожен згенерований контролер. Перезапустіть застосунок і перевірте дві поведінки:

```sh
# no token: rejected before your handler even runs
$ curl -X POST -H "Content-Type: application/json" \
    -d '{"title":"secret","content":"shh"}' localhost:5150/api/notes
# 401 Unauthorized

# with token: goes through
$ curl -X POST -H "Content-Type: application/json" \
    -H "Authorization: Bearer TOKEN" \
    -d '{"title":"secret","content":"shh"}' localhost:5150/api/notes
{"id":1,"created_at":"...","updated_at":"...","title":"secret","content":"shh"}
```

`list`, `get_one`, `update` та `remove` для `notes` досі відкриті для всіх — додайте `auth: auth::JWT` до їхніх сигнатур так само, якщо хочете повністю закрити ресурс.

## Що насправді перевіряється, а що ні

- Секрет JWT і термін його дії зберігаються у `config/development.yaml` під `auth.jwt`. Кожне середовище (`development`, `test`, `production`) отримує власний згенерований секрет — ніколи не використовуйте один секрет у кількох середовищах. Дивіться [Довідник конфігурації](/uk/docs/reference/configuration/) для всіх ключів під `auth:`.
- Токени за замовчуванням підписуються HS512, а налаштований секрет має бути дійсним base64 — згенерована конфігурація робить це за вас, але це важливо, якщо ви колись створюватимете секрет вручну.
- `auth::JWT` перевіряє лише, що токен дійсний і не прострочений; він не перевіряє `is_verified`. Якщо вашому застосунку потрібне правило «електронна пошта має бути підтверджена» як бізнес-правило, перевірте `user.email_verified_at.is_some()` самостійно всередині обробника, так само, як ви шукали користувача за `pid` вище.

## Далі

- [Захистіть маршрут за допомогою JWT](/uk/docs/how-to/jwt-auth/) — повний довідник ендпоінт за ендпоінтом: забутий/скидання пароля, підтвердження електронної пошти, магічні посилання та автентифікація за API-ключем як альтернатива JWT.
- [Довідник конфігурації](/uk/docs/reference/configuration/) — кожен ключ YAML під `auth:` та `mailer:`.
- [Тур](/uk/docs/tutorials/the-tour/) — якщо ви ще не бачили, ознайомтеся з моделями, воркерами та завданнями від початку до кінця.
- [Додайте модель](/uk/docs/how-to/add-model/) — продовжуйте розбудовувати `notes` (або власний ресурс) зі зв'язками та валідацією.
