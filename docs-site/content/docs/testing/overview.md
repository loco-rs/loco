+++
title = "Overview"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

To simplify the testing process, `Loco` provides helpful functions that make writing tests more convenient. Ensure you enable the testing feature in your `Cargo.toml`:

```toml
[dev-dependencies]
loco-rs = { version = "*",  features = ["testing"] }
```

## Testing Helpers Capabilities

### Clean Up Database for Each Test

In some cases, you may want to run tests with a clean dataset, ensuring that each test is independent of others and not affected by previous data. To enable this feature, modify the `dangerously_truncate` option to true in the `config/test.yaml` file under the database section. This setting ensures that Loco truncates all data before each test that implements the boot app.

> ⚠️ Caution: Be cautious when using this feature to avoid unintentional data loss, especially in a production environment.

- When doing it recommended to run all the relevant task in with (serial)[https://crates.io/crates/rstest] crate.
- To decide which tables you want to truncate, add the entity model to the App hook:

```rust
pub struct App;
#[async_trait]
impl Hooks for App {
    //...
    async fn truncate(db: &DatabaseConnection) -> Result<()> {
        // truncate_table(db, users::Entity).await?;
        // truncate_table(db, notes::Entity).await?;
        Ok(())
    }

}
```

### Clean Up Data Before Snapshot Testing

Snapshot testing often involves comparing data structures with dynamic fields such as `created_date`, `id`, `pid`, etc. To ensure consistent snapshots, Loco defines a list of constant data with regex replacements. These replacements can replace dynamic data with placeholders.

Example using [insta](https://crates.io/crates/insta) for snapshots.

in the following example you can use `cleanup_user_model` which clean all user model data.

```rust

#[tokio::test]
#[serial]
async fn can_cerate_user() {
    testing::request::<App, Migrator, _, _>(|request, _ctx| async move {
        // create user test
        with_settings!({
            filters => testing::cleanup_user_model()
        }, {
            assert_debug_snapshot!(current_user_request.text());
        });
    })
    .await;
}

```

You can also use cleanup constants directly, starting with `CLEANUP_`.

### Seeding Data

Refer to the Seed [Seed Documentation](@/docs/the-app/seeding.md).

```rust
#[tokio::test]
#[serial]
async fn is_user_exists() {
    configure_insta!();

    let boot = testing::boot_test::<App, Migrator>().await;
    testing::seed::<App>(&boot.app_context.db).await.unwrap();
    assert!(get_user_by_id(1).ok());

}
```

This documentation provides an in-depth guide on leveraging Loco's testing helpers, covering database cleanup, data cleanup for snapshot testing, and seeding data for tests.
