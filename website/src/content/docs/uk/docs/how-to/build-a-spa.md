---
title: Створюємо типізований React SPA
description: "Використовуйте clientside-режим Loco: фронтенд на Vite + React + TanStack Query, чиї TypeScript-типи генеруються з ваших Rust DTO за допомогою ts-rs, тож зміна схеми ламає збірку фронтенду, а не продакшн."
sidebar:
  order: 18
---

**Мета:** створити односторінковий застосунок поверх вашого Loco-бекенду, де TypeScript-типи походять з ваших Rust-типів — а не з копії, що ведеться вручну і мовчки відстає.

Саме це створює `loco new`, коли ви обираєте **clientside**-ресурси. Якщо у вас уже є застосунок, структура нижче — це те, що потрібно додати.

## 1. Що ви отримуєте

Clientside-застосунок — це один Cargo-проєкт з текою `frontend/` всередині:

```
myapp/
├── src/
│   ├── controllers/         # ваш JSON API
│   └── dtos/                # типи обміну — джерело істини
├── frontend/
│   ├── package.json         # react 19, react-router, @tanstack/react-query
│   ├── vite.config.ts
│   └── src/
│       ├── main.tsx         # QueryClientProvider + RouterProvider
│       ├── routes.tsx       # таблиця маршрутів
│       ├── api/client.ts    # обгортка fetch: bearer-токен, мапінг помилок
│       ├── bindings/        # ЗГЕНЕРОВАНИЙ TypeScript — ніколи не редагуйте вручну
│       ├── auth/            # зберігання токенів, Login, RequireAuth
│       └── pages/
└── config/
```

Дві теки несуть усю ідею: **`src/dtos/`** містить Rust-типи, а **`frontend/src/bindings/`** — їхні TypeScript-еквіваленти, згенеровані автоматично.

## 2. Конвеєр типів

DTO — це звичайна Rust-структура з виведеним (derive) [`ts_rs::TS`](https://docs.rs/ts-rs):

```rust
use ts_rs::TS;

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct PostDto {
    #[ts(type = "number")]
    pub id: i64,
    pub title: String,
    pub status: PostStatus,
    #[ts(type = "string")]
    pub price: Decimal,
    #[ts(type = "string | null")]
    pub published_at: Option<DateTimeWithTimeZone>,
}
```

`#[ts(type = "...")]` — це спосіб зафіксувати форму типу «на дроті», яку `ts-rs` не може вивести самостійно — `i64` є JavaScript-`number`, а `Decimal` перетинає провід як `string`, щоб не втратити точність.

Тримайте DTO окремо від ваших Sea-ORM-сутностей і конвертуйте на межі:

```rust
impl From<crate::models::_entities::posts::Model> for PostDto {
    fn from(m: crate::models::_entities::posts::Model) -> Self {
        Self { id: m.id, title: m.title, status: PostStatus::from(m.status), .. }
    }
}
```

Цей `From` — шов. Ваша схема бази даних може змінюватися, не змінюючи ваш API, а коли ви *хочете* змінити API, компілятор проведе вас за руку.

### Перегенерація біндингів

**`#[ts(export)]` генерує тест.** Файли `.ts` записуються, коли ви запускаєте:

```sh
cargo test
```

Це вся команда — окремого кроку експорту немає і build-скрипта теж немає. Біндинги оновлюються як побічний ефект тестового набору, а це означає, що CI перегенеровує їх під час кожного запуску, і застарілий біндинг проявляється як diff.

Після зміни DTO запустіть `cargo test`, потім перезберіть фронтенд. Поле, яке ви видалили в Rust, тепер є помилкою компіляції TypeScript на кожній сторінці, що його читала.

## 3. Створіть скаффолд для ресурсу

За наявності `frontend/` команда `scaffold` є адаптивною — вона генерує і бекенд, і фронтенд:

```sh
cargo loco generate scaffold post title:string content:text status:enum:draft,published
```

Ви отримуєте звичайні модель, міграцію та контролер, плюс:

| Файл | Що це |
| --- | --- |
| `src/dtos/posts.rs` | `PostDto`, `CreatePost`, `UpdatePost`, enum-и — усе `#[ts(export)]` |
| `frontend/src/api/posts.ts` | типізовані хуки TanStack Query: `useListPosts`, `usePost`, `useCreatePost`, `useUpdatePost`, `useRemovePost` |
| `frontend/src/pages/posts/` | `List`, `Show`, `New`, `Edit` |
| `frontend/src/routes.tsx` | імпорти та маршрути, вбудовані в анкори `// scaffold:imports` і `// scaffold:routes` |

Ці два анкорні коментарі мають залишатися в `routes.tsx`. Саме так генератор знаходить своє місце; якщо ви їх видалите, наступний скаффолд впаде з явною помилкою, а не мовчки згенерує сторінки, до яких ніхто не веде.

Згенеровані хуки самі керують анулюванням кешу, тому створення чи видалення оновлює список без жодного підключення з вашого боку:

```tsx
export function List() {
  const { data, isLoading, isError, error } = useListPosts();
  const removePost = useRemovePost();
  // ...
}
```

## 4. Цикл розробки

Запустіть обидва сервери поруч:

```sh
cargo loco start                 # :5150 — API
cd frontend && pnpm install && pnpm dev   # :5173 — Vite, з HMR
```

Розробляйте проти **`http://localhost:5173`**. Vite проксує `/api` до бекенду, тож браузер бачить один origin і CORS налаштовувати не потрібно:

```ts
// frontend/vite.config.ts
server: { port: 5173, proxy: { '/api': 'http://localhost:5150' } }
```

## 5. Доставте це

Зберіть фронтенд, потім запустіть застосунок:

```sh
cd frontend && pnpm build        # записує frontend/dist/
cargo loco start
```

Loco подає бандл через middleware static, уже налаштований за вас:

```yaml
server:
  middlewares:
    fallback:
      enable: false
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/"
        path: "frontend/dist"
      fallback: "frontend/dist/index.html"
```

Ключ `fallback` усередині `static` — саме те, що дозволяє клієнтській маршрутизації пережити жорстке оновлення сторінки: запит до `/posts/42` не збігається з жодним файлом, тому подається `index.html` і React Router бере керування.

:::caution
`must_exist: true` означає, що застосунок **відмовляється стартувати, доки не існує `frontend/dist`**. На щойно згенерованому clientside-застосунку `cargo loco start` падає до того, як ваш код виконається — це не зламаний застосунок, це відсутня збірка фронтенду. Спершу один раз запустіть `pnpm build`. Те саме стосується CI та вашого Dockerfile: збирайте фронтенд перед запуском бінарника або встановіть `must_exist: false` і миріться з 404, доки цього не зробите.
:::

Щоб доставити один самодостатній бінарник із вбудованим бандлом, дивіться [`embedded_assets`](/uk/docs/how-to/serve-assets/#6-вбудуйте-ресурси-в-бінарник-із-embedded_assets).

## 6. Автентифікація

`frontend/src/api/client.ts` додає JWT до кожного запиту і централізовано обробляє його завершення:

```ts
const token = getToken();
if (token) {
  headers["Authorization"] = `Bearer ${token}`;
}
// ...
if (res.status === 401) {
  clearToken();
  window.location.href = "/login";
}
```

Захист маршрутів — це один компонент: `RequireAuth` рендерить `<Outlet />`, коли токен присутній, і перенаправляє інакше:

```tsx
export function RequireAuth() {
  if (getToken() === null) {
    return <Navigate to="/login" replace />;
  }
  return <Outlet />;
}
```

Скаффолдені маршрути вбудовуються **всередину** гілки `RequireAuth` таблиці маршрутів, що відповідає бекенду: згенеровані контролери вимагають JWT. Серверний бік дивіться у [JWT-автентифікація](/uk/docs/how-to/jwt-auth/).

Згенероване сховище токенів використовує `localStorage`. Це найпростіше, що працює, для застосунку на старт; якщо захист від XSS важливий для вашої моделі загроз, перенесіть токен у httpOnly-cookie і переключіть сервер на cookie-розташування JWT — дивіться [Розташування JWT](/uk/docs/how-to/jwt-locations/).

## 7. Повний приклад

[`examples/reference_spa`](https://github.com/loco-rs/loco/tree/master/examples/reference_spa) у репозиторії Loco — повноцінний робочий застосунок, побудований саме так — DTO з enum-ами та decimal, згенеровані біндинги, типізовані хуки та чотири скаффолдені сторінки. Це застосунок, який відтворюють `loco new` + `generate scaffold`, і він покривається тестовим набором, тож залишається чесним.

## Пов'язане

- [Обслуговування статичних та SPA-ресурсів](/uk/docs/how-to/serve-assets/) — middleware static у деталях
- [Додаємо контролер](/uk/docs/how-to/add-controller/) — API, який викликає SPA
- [Використовуємо генератори](/uk/docs/how-to/use-generators/) — кожен генератор і прапорець
- [Розгортання](/uk/docs/how-to/deploy/) — не забудьте зібрати фронтенд у своєму пайплайні
