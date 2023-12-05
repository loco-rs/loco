{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: tests/tasks/{{ file_name }}.rs
skip_exists: true
message: "Tests for task `{{module_name}}` was added successfully. Run `cargo run test`."
injections:
- into: tests/tasks/mod.rs
  append: true
  content: "pub mod {{ file_name }};"
---
use {{pkg_name}}::app::App;
use loco_rs::testing;

use loco_rs::boot::run_task;
use migration::Migrator;
use serial_test::serial;
use std::collections::BTreeMap;

#[tokio::test]
#[serial]
async fn test_can_run_{{name | snake_case}}() {
    let boot = testing::boot_test::<App, Migrator>().await.unwrap();

    let vars = BTreeMap::new();

    assert!(
        run_task::<App>(&boot.app_context, Some(&"{{name}}".to_string()), &vars)
            .await
            .is_ok()
    );
}
