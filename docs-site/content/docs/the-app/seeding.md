+++
title = "Seeding"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 16
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

`Loco` has a built-in 'seeds' feature that makes the process quick and easy. This is especially useful when reloading the database frequently in development and test environments. It's easy to get started with this feature

`Loca` comes equipped with a convenient `seeds` feature, streamlining the process for quick and easy database reloading. This functionality proves especially invaluable during frequent resets in development and test environments. Let's explore how to get started with this feature:

## Steps for creating a new seed

### 1. Creating a new seed file

Navigate to `src/fixtures` and create a new seed file. For instance:

```
src/
  fixtures/
    users.yaml
```

In this yaml file, enlist a set of database records for insertion. Each record should encompass the mandatory database fields, based on your database constraints. Optional values are at your discretion. Suppose you have a database DDL like this:

```sql
CREATE TABLE public.users (
	id serial4 NOT NULL,
	email varchar NOT NULL,
	"password" varchar NOT NULL,
	reset_token varchar NULL,
	created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
	CONSTRAINT users_email_key UNIQUE (email),
	CONSTRAINT users_pkey PRIMARY KEY (id)
);
```

The mandatory fields include `id`, `password`, `email`, and `created_at`. The reset token can be left empty. Your migration content file should resemble the following:

```yaml
---
- id: 1
  email: user1@example.com
  password: "$2b$12$gf4o2FShIahg/GY6YkK2wOcs8w4.lu444wP6BL3FyjX0GsxnEV6ZW"
  created_at: "2023-11-12T12:34:56.789"
- id: 2
  pid: 22222222-2222-2222-2222-222222222222
  email: user2@example.com
  reset_token: "SJndjh2389hNJKnJI90U32NKJ"
  password: "$2b$12$gf4o2FShIahg/GY6YkK2wOcs8w4.lu444wP6BL3FyjX0GsxnEV6ZW"
  created_at: "2023-11-12T12:34:56.789"
```

### Connect the seed

Integrate your seed into the app's Hook implementations by following these steps:

1. Navigate to your app's Hook implementations.
2. Add the seed within the seed function implementation. Here's an example in Rust:

```rs
impl Hooks for App {
    // Other implementations...

    async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(db, &base.join("users.yaml").display().to_string()).await?;
        Ok(())
    }
}

```

This implementation ensures that the seed is executed when the seed function is called. Adjust the specifics based on your application's structure and requirements.

## Running the Seed Process

The seed process is not executed automatically. You can trigger the seed process either through a task or during testing.

### Using a Task

1. Create a seeding task by following the instructions in the [Task Documentation](@/docs/the-app/task.md).
2. Configure the task to execute the `seed` function, as demonstrated in the example below:

```rust
use std::collections::BTreeMap;

use async_trait::async_trait;
use loco_rs::{
    app::AppContext,
    db,
    task::{Task, TaskInfo},
    Result,
};
use sea_orm::EntityTrait;

use crate::{app::App, models::_entities::users};

pub struct SeedData;
#[async_trait]
impl Task for SeedData {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed".to_string(),
            detail: "Seeding data".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>) -> Result<()> {
        let path = std::path::Path::new("src/fixtures");
        db::run_app_seed::<App>(&app_context.db, path).await
    }
}
```

### Using a Test

1. Enable the testing feature in [dev-dependencies]. Refer to the [Testing Overview](@/docs/testing/overview.md) in the task documentation.

2. In your test section, follow the example below:

```rust
#[tokio::test]
#[serial]
async fn handle_create_with_password_with_duplicate() {

    let boot = testing::boot_test::<App, Migrator>().await;
    testing::seed::<App>(&boot.app_context.db).await.unwrap();
    assert!(get_user_by_id(1).ok());
}
```
