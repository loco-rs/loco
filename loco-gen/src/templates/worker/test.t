{% set module_name = name |  snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "tests/workers/{{module_name}}.rs"
skip_exists: true
message: "Test for worker `{{struct_name}}` was added successfully. Run `cargo test`."
injections:
- into: tests/workers/mod.rs
  append: true
  content: "pub mod {{ name |  snake_case }};"
---
use loco_rs::{bgworker::BackgroundWorker, testing::prelude::*};
use {{pkg_name}}::{
    app::App,
    workers::{{module_name}}::{Worker, WorkerArgs},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_run_{{module_name}}_worker() {
    let boot = boot_test::<App>().await.unwrap();

    // Execute the worker ensuring that it operates in 'ForegroundBlocking' mode, which prevents the addition of your worker to the background
    assert!(
        Worker::perform_later(&boot.app_context,WorkerArgs {})
            .await
            .is_ok()
    );
    // Include additional assert validations after the execution of the worker
}
