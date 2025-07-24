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
use serde::{Deserialize, Serialize};
use axum::debug_handler;

use crate::models::_entities::{{file_name | plural}}::{ActiveModel, Entity, Model};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Params {
    {% for column in columns -%}
    {%- if column.2 == "IntegerNull" -%}
    pub {{column.0}}: Option<i32>,
    {%- else -%}
    pub {{column.0}}: {{column.1}},
    {%- endif %}
    {% endfor -%}
}

impl Params {
    fn update(&self, item: &mut ActiveModel) {
      {% for column in columns -%}
      {%- if "Vec<" in column.1 -%}
      item.{{column.0}} = Set(self.{{column.0}}.clone());
      {%- elif column.2 == "IntegerNull" -%}
      item.{{column.0}} = Set(self.{{column.0}});
      {%- elif "i32" in column.1 or "i64" in column.1 or "i16" in column.1 or "Uuid" in column.1 or "f32" in column.1 or "f64" in column.1 or "Decimal" in column.1 or "bool" in column.1 or "Date" in column.1 or "DateTime" in column.1 or "DateTimeWithTimeZone" in column.1 -%}
      item.{{column.0}} = Set(self.{{column.0}});
      {%- else -%}
      item.{{column.0}} = Set(self.{{column.0}}.clone());
      {%- endif %}
      {% endfor -%}
    }
}

async fn load_item(ctx: &AppContext, id: i32) -> Result<Model> {
    let item = Entity::find_by_id(id).one(&ctx.db).await?;
    item.ok_or_else(|| Error::NotFound)
}

#[debug_handler]
pub async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(Entity::find().all(&ctx.db).await?)
}

#[debug_handler]
pub async fn add(State(ctx): State<AppContext>, Json(params): Json<Params>) -> Result<Response> {
    let mut item = ActiveModel {
        ..Default::default()
    };
    params.update(&mut item);
    let item = item.insert(&ctx.db).await?;
    format::json(item)
}

#[debug_handler]
pub async fn update(
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
    Json(params): Json<Params>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    let mut item = item.into_active_model();
    params.update(&mut item);
    let item = item.update(&ctx.db).await?;
    format::json(item)
}

#[debug_handler]
pub async fn remove(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {
    load_item(&ctx, id).await?.delete(&ctx.db).await?;
    format::empty()
}

#[debug_handler]
pub async fn get_one(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {
    format::json(load_item(&ctx, id).await?)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/{{file_name | plural}}/")
        .add("/", get(list))
        .add("/", post(add))
        .add("{id}", get(get_one))
        .add("{id}", delete(remove))
        .add("{id}", put(update))
        .add("{id}", patch(update))
}
