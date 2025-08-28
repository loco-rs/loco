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
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

#[debug_handler]
pub async fn index(State(_ctx): State<AppContext>) -> Result<Response> {
    format::empty()
}

{% for action in actions -%}
#[debug_handler]
pub async fn {{action}}(State(_ctx): State<AppContext>) -> Result<Response> {
    format::empty()
}

{% endfor -%}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/{{file_name | plural}}/")
        .add("/", get(index))
        {%- for action in actions %}
        .add("{{action}}", get({{action}}))
        {%- endfor %}
}
