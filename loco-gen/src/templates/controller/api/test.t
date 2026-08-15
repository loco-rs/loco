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
async fn can_get_{{ name | plural | snake_case }}() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/{{ name | plural | snake_case }}").await;
{% if auth %}        // this route takes an `auth::JWT` extractor, so an anonymous request is
        // rejected. To exercise the handler itself, register and log in a user
        // first and send `Authorization: Bearer <token>` -- see
        // https://loco.rs/docs/how-to/jwt-auth
        assert_eq!(res.status_code(), 401);
{% else %}        assert_eq!(res.status_code(), 200);

        // you can assert content like this:
        // assert_eq!(res.text(), "content");
{% endif %}    })
    .await;
}

{% for action in actions -%}
#[tokio::test]
#[serial]
async fn can_get_{{action}}() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/{{ name | plural | snake_case }}/{{action}}").await;
        assert_eq!(res.status_code(), {% if auth %}401{% else %}200{% endif %});
    })
    .await;
}

{% endfor -%}
