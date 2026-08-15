---
title: Write a one-off task
description: Implement the Task trait, register it, and run it with cargo loco task.
sidebar:
  order: 23
---

Goal: run an ad-hoc, CLI-invokable operation (data fix, report, one-time migration) with typed access to your app's `AppContext` — without building a UI for it. Tasks can also be invoked on a schedule, see [Schedule recurring jobs](/docs/how-to/schedule-jobs).

## 1. Generate a task

```sh
cargo loco generate task user_report
```

This creates `src/tasks/user_report.rs`, adds `pub mod user_report;` to `src/tasks/mod.rs`, and registers it in `src/app.rs`:

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

## 2. Implement the logic

The `Task` trait has two parts: `task()` describes the task (its name and a help blurb shown when listing tasks), and `run()` does the work with access to `AppContext` and CLI arguments:

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

(Adapted from `examples/demo/src/tasks/user_create.rs`.)

`vars.cli_arg("key")` reads a `key:value` pair passed on the command line; it returns a `Result<&str>`, so missing required arguments become a clear task error rather than a panic (and you own no allocation — call `.to_string()` when you need one).

## 3. Confirm registration

The generator injects this automatically — but if you write a task by hand, register it yourself in `register_tasks`:

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

Registering a task under a name that's already taken replaces the previous one — the registry is keyed by task name.

## 4. Run it

```sh
cargo loco task user:create email:user@example.com name:"John Doe" password:secret
```

General form:

```sh
cargo loco task <TASK_NAME> [KEY:VALUE ...]
```

## 5. List all registered tasks

```sh
cargo loco task
```

Running `task` with no name lists every task currently registered via `register_tasks` (not a history of past runs) — each with its `name` and `detail`.

## Reference

- Run tasks on a schedule instead of manually: [Schedule recurring jobs](/docs/how-to/schedule-jobs)
- `cargo loco task` / `cargo loco generate task` flags: [CLI reference](/docs/reference/cli)
