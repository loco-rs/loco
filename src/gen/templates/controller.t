{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: src/controllers/{{ file_name }}.rs
skip_exists: true
message: "Controller `{{module_name}}` was added successfully."
injections:
- into: src/controllers/mod.rs
  append: true
  content: "pub mod {{ file_name }};"
- into: src/app.rs
  after: "AppRoutes::"
  content: "            .add_route(controllers::{{ file_name }}::routes())"
---
#![allow(clippy::unused_async)]
use axum::{
    extract::State,
    routing::{get, post},
};
use rustyrails::{
    app::AppContext,
    controller::{format, Routes},
    Result,
};

pub async fn echo(req_body: String) -> String {
    req_body
}

pub async fn hello(State(_ctx): State<AppContext>) -> Result<String> {
    // do something with context (database, etc)
    format::text("hello")
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("{{ name | snake_case }}")
        .add("/", get(hello))
        .add("/echo", post(echo))
}
