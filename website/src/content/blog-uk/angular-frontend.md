---
title: Створення фронтенд-сайту за допомогою Angular
description: Налаштувати Loco-застосунок для обслуговування клієнтського Angular-застосунку легко. Дізнайтеся, як сконфігурувати та розгорнути повностековий Angular-застосунок з Loco.
pubDate: 2024-01-25
updatedDate: 2024-01-25
authors:
  - limpidcrypto
---

## Огляд

1. Створіть новий SaaS-проєкт
2. Відредагуйте `.devcontainer/Dockerfile`
3. Відкрийте проєкт знову в Dev Container
4. Видаліть каталог frontend
5. Згенеруйте новий Angular-фронтенд
6. Зберіть фронтенд
7. Відредагуйте `config/development.yml`
8. Запустіть Loco

## Створення нового SaaS-проєкту

1. Виконайте `loco new`, щоб створити новий проєкт
2. Пройдіть інструкції до моменту, коли потрібно вирішити, який тип проєкту створити
3. Оберіть "SaaS app (with DB and user auth)"

## Редагування ".devcontainer/Dockerfile"

1. Відкрийте `.devcontainer/Dockerfile`
2. Замініть вміст на наступний:

```Dockerfile
FROM mcr.microsoft.com/vscode/devcontainers/rust:0-1

# Встановлюємо postgresql-client та sea-orm-cli
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends postgresql-client \
    && cargo install sea-orm-cli \
    && chown -R vscode /usr/local/cargo

# Встановлюємо Node.js та npm
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs
# Встановлюємо Angular CLI
RUN npm install -g @angular/cli

COPY .env /.env
```

Цей Dockerfile надасть вам усе необхідне для розробки Loco-застосунку з Angular-фронтендом.

## Повторне відкриття проєкту в Dev Container

З VSCode відкрити та запустити проєкт у Dev Container надзвичайно просто.

1. Натисніть `Crtl + Shift + P`
2. Оберіть `Dev Containers: Repopen in Container`
3. VSCode відкриє проєкт у dev-контейнері. Перша збірка може тривати деякий час.
4. Видаліть наявний каталог `frontend`

Loco постачається з Vite React-фронтендом. Ми можемо видалити весь каталог, оскільки Angular CLI налаштує все, що нам потрібно.

## Генерація нового Angular-фронтенду

1. З кореня проєкту виконайте `ng new frontend`, щоб створити новий Angular-проєкт
2. Пройдіть інструкції

## Збірка фронтенду

1. Виконайте `ng build`, щоб зібрати Angular-фронтенд

## Редагування "config/development.yml"

Як ви могли помітити, Angular зібрав фронтенд у `frontend/dist/frontend/browser`. Тепер нам потрібно налаштувати Loco, щоб він брав зібраний фронтенд звідти.

1. Відкрийте `config/development.yml`
2. Задайте конфігурації шляху до зібраного фронтенду:

   a. `server.middlewares.static.folder.path: "frontend/dist/frontend/browser"`

   b. `server.middlewares.static.fallback: "frontend/dist/frontend/browser/index.html"`

## Запуск Loco

1. Запустіть Loco командою `cargo loco start`
2. Відкрийте http://localhost:5150/

Тепер ви маєте побачити стартовий сайт Angular :smile:
