{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: src/controllers/{{ file_name }}.rs
skip_exists: true
message: "Controller `{{module_name}}` was added successfully.{% if auth %} Its routes require a JWT.{% endif %}"
injections:
- into: src/controllers/mod.rs
  append: true
  content: "pub mod {{ file_name }};"
{% if has_routes_file -%}
- into: src/routes.rs
  after: "AppRoutes::"
  content: |2-
            .add_route(
                Routes::new()
                    .prefix("api/{{file_name | plural}}/")
                    .add("/", get(crate::controllers::{{ file_name }}::index))
                    {%- for action in actions %}
                    .add("{{action.name}}", {{action.verb}}(crate::controllers::{{ file_name }}::{{action.name}}))
                    {%- endfor %}
            )
{% else -%}
- into: src/app.rs
  after: "AppRoutes::"
  content: "            .add_route(controllers::{{ file_name }}::routes())"
{% endif -%}
---
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

#[debug_handler]
pub async fn index({% if auth %}_auth: auth::JWT, {% endif %}State(_ctx): State<AppContext>) -> Result<Response> {
    format::empty()
}

{% for action in actions -%}
#[debug_handler]
pub async fn {{action.name}}({% if auth %}_auth: auth::JWT, {% endif %}State(_ctx): State<AppContext>) -> Result<Response> {
    format::empty()
}

{% endfor -%}

{% if not has_routes_file -%}
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/{{file_name | plural}}/")
        .add("/", get(index))
        {%- for action in actions %}
        .add("{{action.name}}", {{action.verb}}({{action.name}}))
        {%- endfor %}
}
{% endif -%}
