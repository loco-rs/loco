---
title: Пишемо разовий таск
description: Реалізуйте трейт Task, зареєструйте його та запускайте через cargo loco task.
sidebar:
  order: 23
---

Мета: виконати разову операцію, яку можна викликати з CLI (виправлення даних, звіт, одноразову міграцію), з типізованим доступом до `AppContext` вашого застосунку — без розробки для цього UI. Таски також можна викликати за розкладом, дивіться [Планування повторюваних завдань](/uk/docs/how-to/schedule-jobs/).

## 1. Створюємо таск

```sh
cargo loco generate task user_report
```

Це створить `src/tasks/user_report.rs`, додасть `pub mod user_report;` до `src/tasks/mod.rs` і зареєструє його в `src/app.rs`:

```rust
use loco_rs::prelude::*;

pub struct UserReport;
#[async_trait]
impl Task for UserReport {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user_report".to_string(),
            detail: "Task generator".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, _vars: &task::Vars) -> Result<()> {
        println!("Task UserReport generated");
        Ok(())
    }
}
```

## 2. Реалізуємо логіку

Трейт `Task` складається з двох частин: `task()` описує таск (його назву та довідковий текст, що показується під час переліку тасків), а `run()` виконує роботу з доступом до `AppContext` і аргументів CLI:

```rust
use loco_rs::prelude::*;

use crate::{mailers::auth::AuthMailer, models::_entities::users, models::users::RegisterParams};

pub struct UserCreate;
#[async_trait]
impl Task for UserCreate {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user:create".to_string(),
            detail: "Create a new user with email, name, and password.\n\
                     Usage: cargo loco task user:create email:user@example.com name:\"John Doe\" password:\"secret\""
                .to_string(),
        }
    }

    async fn run(&self, app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        let email = vars.cli_arg("email").map_err(|_| Error::string("email is mandatory"))?;
        let name = vars.cli_arg("name").map_err(|_| Error::string("name is mandatory"))?;
        let password = vars.cli_arg("password").map_err(|_| Error::string("password is mandatory"))?;

        let register_params = RegisterParams {
            email: email.to_string(),
            password: password.to_string(),
            name: name.to_string(),
        };
        let user = users::Model::create_with_password(&app_context.db, &register_params).await?;

        AuthMailer::send_welcome(app_context, &user).await?;

        println!("user created: {}", user.email);
        Ok(())
    }
}
```

(Адаптовано з `examples/demo/src/tasks/user_create.rs`.)

`vars.cli_arg("key")` зчитує пару `key:value`, передану в командному рядку; вона повертає `Result<&str>`, тож відсутній обов'язковий аргумент перетворюється на зрозумілу помилку таска, а не на паніку (і вам не належить жодне виділення пам'яті — викличте `.to_string()`, коли потрібен рядок у власності).

## 3. Перевіряємо реєстрацію

Генератор вписує це автоматично — але якщо ви пишете таск вручну, зареєструйте його самі в `register_tasks`:

```rust
// src/app.rs
impl Hooks for App {
    // ..
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::user_create::UserCreate);
    }
    // ..
}
```

Реєстрація таска під уже зайнятою назвою замінює попередній — реєстр ключується за назвою таска.

## 4. Запускаємо

```sh
cargo loco task user:create email:user@example.com name:"John Doe" password:secret
```

Загальна форма:

```sh
cargo loco task <TASK_NAME> [KEY:VALUE ...]
```

## 5. Перелічуємо всі зареєстровані таски

```sh
cargo loco task
```

Запуск `task` без назви перелічує всі таски, зареєстровані наразі через `register_tasks` (це не історія минулих запусків) — кожен із його `name` та `detail`.

## Довідка

- Запуск тасків за розкладом замість ручного виклику: [Планування повторюваних завдань](/uk/docs/how-to/schedule-jobs/)
- Прапорці `cargo loco task` / `cargo loco generate task`: [Довідник CLI](/uk/docs/reference/cli/)
