+++
title = "Model"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

Testing models can be intricate, especially when reading and inserting data simultaneously during tests. This complexity arises from dynamic fields such as `id` or `created_at`, which can introduce inconsistencies into our tests.

## Initializing the App

Initiate your tests with the following command:

```rust
testing::boot_test::<App, Migrator>().await;
```

During this initialization:

1. Set up your application in **test** mode, loading the `config/test.yaml` file.
2. Obtain the application context with the **DB** instance for interacting with the model.
3. Perform table truncation for a clean state. [documentation here](@/docs/testing/overview.md#clean-up-data-before-snapshot-testing)
4. Execute tests with seeded data. [documentation here](@/docs/testing/overview.md#seeding-data)

Consider the example below, where we test the existence of a user in the database:

```rust
#[tokio::test]
#[serial]
async fn can_find_by_email() {

    let boot = testing::boot_test::<App, Migrator>().await;
    testing::seed::<App>(&boot.app_context.db).await.unwrap();

    let existing_user = Model::find_by_email(&boot.app_context.db, "user1@example.com").await;
    let non_existing_user_results =
        Model::find_by_email(&boot.app_context.db, "un@existing-email.com").await;

    assert!(existing_user);
    assert_eq!(non_existing_user_results, false);
}
```
