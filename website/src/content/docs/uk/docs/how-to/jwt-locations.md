---
title: Налаштовуємо, де Loco шукає JWT
description: Задайте розташування JWT-токена — Bearer-заголовок, query-параметр або cookie — як одне місце чи як список з fallback.
sidebar:
  order: 42
---

Мета: керувати тим, де екстрактори `JWT` і `JWTWithUser<T>` шукають токен у вхідному запиті — у заголовку `Authorization` (за замовчуванням), у query-параметрі, у cookie або за списком кількох місць із fallback.

Це налаштування — `auth.jwt.location` у вашому конфігураційному файлі. Воно стосується лише JWT-екстракторів (`auth::JWT`, `auth::JWTWithUser<T>`); воно **не діє** на `auth::ApiToken<T>`, який завжди зчитує `Bearer`-заголовок незалежно від цієї конфігурації — дивіться [Захищаємо маршрут за допомогою API-ключа](/uk/docs/how-to/api-key-auth/#3-надсилаємо-ключ-як-bearer-токен).

Якщо ви ще не налаштували JWT-автентифікацію, почніть із [Захищаємо маршрут за допомогою JWT](/uk/docs/how-to/jwt-auth/); ця сторінка висвітлює лише ключ `location`.

## За замовчуванням: конфігурація не потрібна

Якщо повністю пропустити `location`, Loco зчитує токен із заголовка `Authorization: Bearer <token>`:

```yaml
auth:
  jwt:
    secret: "<%= get_env(name='JWT_SECRET') %>"
    expiration: 604800
    # location не задано => Bearer-заголовок
```

## Одне місце

Задайте `location` як одну мапу з тегом `from:`. Три варіанти:

**Bearer-заголовок** (еквівалент типового значення, виписаний явно):

```yaml
auth:
  jwt:
    location:
      from: Bearer
    secret: "<%= get_env(name='JWT_SECRET') %>"
    expiration: 604800
```

**Query-параметр** — зчитує іменоване значення з query-рядка, наприклад для посилань, які не можуть нести кастомні заголовки (посилання підтвердження email, handshake WebSocket):

```yaml
auth:
  jwt:
    location:
      from: Query
      name: token
    secret: "<%= get_env(name='JWT_SECRET') %>"
    expiration: 604800
```

```sh
curl 'http://127.0.0.1:5150/api/protected?token=<TOKEN>'
```

**Cookie** — зчитує іменовану cookie, наприклад для server-rendered застосунків із session-style cookies:

```yaml
auth:
  jwt:
    location:
      from: Cookie
      name: auth_token
    secret: "<%= get_env(name='JWT_SECRET') %>"
    expiration: 604800
```

```sh
curl 'http://127.0.0.1:5150/api/protected' --cookie 'auth_token=<TOKEN>'
```

## Кілька місць (перевіряються по порядку)

Задайте `location` як YAML-список замість однієї мапи. Loco пробує кожне місце у зазначеному порядку й використовує перше, яке дає токен — корисно, коли, скажімо, браузерні клієнти надсилають cookie, а API-клієнти — Bearer-заголовок:

```yaml
auth:
  jwt:
    location:
      - from: Cookie
        name: auth_token
      - from: Query
        name: token
      - from: Bearer
    secret: "<%= get_env(name='JWT_SECRET') %>"
    expiration: 604800
```

З цією конфігурацією запит приймається, якщо несе чинний токен у cookie `auth_token`, **або** (якщо її немає) у query-параметрі `token`, **або** (якщо обох немає) у заголовку `Authorization: Bearer` — перевіряються саме в такому порядку. Якщо жодне з налаштованих місць не дає токена, запит відхиляється з `401 Unauthorized`.

## Перевіряємо, що працює

Для конфігурації `Multiple`, як-от вище, перевірте кожне місце окремо:

```sh
# через cookie
curl 'http://127.0.0.1:5150/api/protected' --cookie 'auth_token=<TOKEN>'

# через query-параметр
curl 'http://127.0.0.1:5150/api/protected?token=<TOKEN>'

# через Bearer-заголовок
curl 'http://127.0.0.1:5150/api/protected' --header 'Authorization: Bearer <TOKEN>'
```

Кожен має успішно пройти незалежно; запит без токена в будь-якому з трьох місць має повертати `401 Unauthorized`.

## Пов'язане

- [Захищаємо маршрут за допомогою JWT](/uk/docs/how-to/jwt-auth/) — екстрактори, якими керує це налаштування, і як генерувати токени.
- [Захищаємо маршрут за допомогою API-ключа](/uk/docs/how-to/api-key-auth/) — окремий механізм, що завжди читає Bearer-заголовок, на який це налаштування не впливає.
- [Довідник конфігурації](/uk/docs/reference/configuration/#auth) — повна таблиця ключів `auth.jwt`, включно зі структурами `JWTLocation`/`JWTLocationConfig`.
