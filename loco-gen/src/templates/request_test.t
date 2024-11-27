{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: tests/requests/{{ file_name }}.rs
skip_exists: true
message: "Tests for controller `{{module_name}}` was added successfully. Run `cargo test`."
injections:
- into: tests/requests/mod.rs
  append: true
  content: "pub mod {{ file_name }};"
---
use {{pkg_name}}::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn can_get_echo() {
    request::<App, _, _>(|request, _ctx| async move {
        let payload = serde_json::json!({
            "foo": "bar",
        });

        let res = request.post("/{{ name | snake_case }}/echo").json(&payload).await;
        assert_eq!(res.status_code(), 200);
        assert_eq!(res.text(), serde_json::to_string(&payload).unwrap());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_request_root() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/{{ name | snake_case }}").await;
        assert_eq!(res.status_code(), 200);
        assert_eq!(res.text(), "hello");
    })
    .await;
}
