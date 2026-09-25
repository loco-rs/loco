---
title: Створення фронтенд-сайту
description: Швидко створіть REST API з Loco, а потім побудуйте React-фронтенд-застосунок для роботи з ним. Дізнайтеся про генератори, налаштування обслуговування статичних ресурсів та клієнтські застосунки з Loco.
pubDate: 2023-12-14
updatedDate: 2023-12-14
authors:
  - team-loco
---

## Огляд

Цей посібник містить вичерпний покроковий огляд використання `Loco` для створення застосунку-списку справ (Todo) з REST API та React-фронтендом. Описані кроки охоплюють усе — від створення проєкту до розгортання.

Приклад репозиторію можна переглянути [тут](https://github.com/loco-rs/todo-list-example)

Ключові кроки:

- Створення Loco-проєкту на основі SaaS-стартера
- Налаштування Vite-фронтенду з React
- Конфігурація Loco для обслуговування статичних ресурсів фронтенду
- Реалізація моделі/контролера Notes у REST API
- Перезавантаження сервера та фронтенду під час розробки
- Розгортання сайту в продакшн

## Вибір SaaS-стартера

Для початку виконайте наступну команду, щоб створити новий Loco-застосунок на основі SaaS-стартера:

```sh
& loco new
✔ ❯ App name? · todolist
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)

🚂 Loco app generated successfully in:
/tmp/todolist
```

Пройдіть підказки, щоб указати назву застосунку (наприклад, todolist) та обрати варіант SaaS-застосунку.

Після генерації застосунку переконайтеся, що у вас є всі необхідні ресурси, виконавши:

```
$ cd todolist
$ cargo loco doctor
✅ SeaORM CLI is installed
✅ DB connection: success
✅ Redis connection: success
```

Перевірте, що SeaORM CLI встановлено, а з'єднання з базою даних та Redis успішні. Якщо якийсь із ресурсів недоступний, зверніться до [швидкого туру](@/docs/tutorials/your-first-app.md) для усунення проблем.

Коли `cargo loco doctor` покаже, що всі перевірки пройдено, запустіть сервер:

```
$ cargo loco start
   Updating crates.io index
   .
   .
   .

                      ▄     ▀
                                 ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
 ██████  █████   ███ █████   ███ █████   ███ ▀█
 ██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
 ██████  █████   ███ █████       █████   ███ ████▄
 ██████  █████   ███ █████   ▄▄▄ █████   ███ █████
 ██████  █████   ███  ████   ███ █████   ███ ████▀
   ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
       ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
                https://loco.rs

environment: development
   database: automigrate
     logger: debug
      modes: server

listening on port 5150
```

## Створення фронтенду

Для фронтенду ми використаємо [Vite](https://vitejs.dev/guide/) з React. У каталозі `todolist` виконайте:

```sh
$ npm create vite@latest
Need to install the following packages:
  create-vite@5.1.0
Ok to proceed? (y) y
✔ Project name: … frontend
✔ Select a framework: › React
✔ Select a variant: › JavaScript
```

Пройдіть підказки, вказавши `frontend` як назву проєкту.

Перейдіть до каталогу фронтенду та встановіть залежності:

```
$ cd todolist/frontend
$ pnpm install
```

Запустіть сервер розробки:

```sh
$ pnpm dev
```

### Обслуговування статичних ресурсів у Loco

По-перше, перемістимо всі наші REST API-ендпоінти під префікс `/api`. Для цього відкрийте `src/app.rs` та в hook-функції `routes` додайте `.prefix("/api")` до типових маршрутів.
```rust
fn routes() -> AppRoutes {
    AppRoutes::with_default_routes()
        .prefix("/api")
        .add_route(controllers::notes::routes())
}
```

Зберіть фронтенд для продакшну:

```sh
pnpm build
```

У каталозі `frontend` буде створено каталог `dist`. Оновіть файл `config/development.yaml` у головному каталозі, щоб додати static-middleware:

```yaml
server:
  middlewares:
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/"
        path: "frontend/dist"
      fallback: "frontend/dist/index.html"
```

Тепер запустіть Loco-сервер знову, і ви побачите, що фронтенд-застосунок обслуговується через Loco
```sh
$ cargo loco start
```

Якщо ви бачите типову fallback-сторінку, вам потрібно вимкнути fallback-middleware. Типовий fallback має пріоритет над обробником статики, тому якщо він увімкнений, статичний контент обслуговуватися не буде. Вимкнути його можна так:

```yaml
server:
  middlewares:
    fallback:
      enable: false
    static:
      ...
```

# Розробка UI

Встановіть `react-router-dom`, `react-query` та `axios`

```sh
$ pnpm install react-router-dom react-query axios
```

1. Скопіюйте [main.jsx](https://github.com/loco-rs/todo-list-example/blob/main/frontend/src/main.jsx) до frontend/src/main.jsx.
2. Скопіюйте [App.jsx](https://github.com/loco-rs/todo-list-example/blob/main/frontend/src/App.jsx) до frontend/src/App.jsx.
3. Скопіюйте [App.css](https://github.com/loco-rs/todo-list-example/blob/main/frontend/src/App.css) до frontend/src/App.css.

Тепер запустіть сервер `cargo loco start` та UI через `pnpm dev` у каталозі фронтенду — і починайте додавати свої справи до списку!

## Покращення розробки

Використовуйте [cargo-watch](https://crates.io/crates/cargo-watch) для гарячого перезавантаження сервера:

```sh
$ cargo watch --ignore "frontend" -x check -s 'cargo run start'
```

Тепер будь-які зміни у вашому Rust-коді автоматично перезавантажать сервер, а будь-які зміни у Vite-фронтенді перезавантажать фронтенд-застосунок.

## Розгортання в продакшн

У каталозі `frontend` виконайте `pnpm build`. Після успішної збірки перейдіть до Loco-сервера та виконайте `cargo loco start`. Loco обслуговуватиме статичні файли фронтенду безпосередньо з сервера.

### Підготовка Docker-образу

Виконайте `cargo loco generate deployment` та оберіть Docker як тип розгортання:

```sh
$ cargo loco generate deployment
✔ ❯ Choose your deployment · Docker
added: "Dockerfile"
added: ".dockerignore"
```

Loco додасть файл `Dockerfile` та `.dockerignore`. Зверніть увагу, що Loco виявляє статичні ресурси та включає їх до складу образу.

Зберіть контейнер:

```sh
$ docker build . -t loco-todo-list
```

Тепер запустіть контейнер:

```sh
$ docker run -e LOCO_ENV=production -p 5150:5150 loco-todo-list start
```
