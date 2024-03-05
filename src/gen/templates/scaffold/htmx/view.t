{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: src/views/{{file_name}}.rs
skip_exists: true
message: "{{file_name}} view was added successfully."
injections:
- into: src/views/mod.rs
  append: true
  content: "pub mod {{ file_name }};"
---
use loco_rs::prelude::*;

use crate::models::_entities::{{file_name | plural}};

pub fn list(v: impl ViewRenderer, items: Vec<{{file_name | plural}}::Model>) -> Result<impl IntoResponse> {
    format::render().view(&v, "{{file_name}}/list.html", serde_json::json!({"items": items}))
}

pub fn show(v: impl ViewRenderer, item: {{file_name | plural}}::Model) -> Result<impl IntoResponse> {
    format::render().view(&v, "{{file_name}}/show.html", serde_json::json!({"item": item}))
}

pub fn create(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::render().view(&v, "{{file_name}}/create.html", serde_json::json!({}))
}

pub fn edit_form(v: impl ViewRenderer, item: {{file_name | plural}}::Model) -> Result<impl IntoResponse> {
    format::render().view(&v, "{{file_name}}/edit.html", serde_json::json!({"item": item}))
}
