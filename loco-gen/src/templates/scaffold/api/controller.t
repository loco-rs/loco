to: src/controllers/{{ snake_plural }}.rs
skip_exists: true
message: "Controller `{{ pascal_singular }}` was added successfully."
injections:
- into: src/controllers/mod.rs
  append: true
  content: "pub mod {{ snake_plural }};"
- into: src/app.rs
  after: "AppRoutes::"
  content: "            .add_route(controllers::{{ snake_plural }}::routes())"
---
#![allow(clippy::unused_async)]
use axum::http::StatusCode;
use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder};
use serde::Deserialize;

use crate::{
    dtos::{
        common::{ApiError, Page},
        {{ snake_plural }}::{Create{{ pascal_singular }}, {{ pascal_singular }}Dto, Update{{ pascal_singular }}},
    },
    models::_entities::{{ snake_plural }}::{ActiveModel, Column, Entity},
};

fn default_page() -> u64 {
    1
}

fn default_per_page() -> u64 {
    25
}

#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_per_page")]
    pub per_page: u64,
}

/// Build a 404 response shaped as the [`ApiError`] envelope.
fn not_found(message: &str) -> Response {
    let error = ApiError {
        code: "not_found".to_string(),
        message: message.to_string(),
        details: None,
    };
    (StatusCode::NOT_FOUND, Json(error)).into_response()
}

#[debug_handler]
async fn list(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Query(params): Query<ListParams>,
) -> Result<Json<Page<{{ pascal_singular }}Dto>>> {
    let page = params.page.max(1);
    let per_page = params.per_page.max(1);

    let query = Entity::find().order_by_asc(Column::Id);

    let paginator = query.paginate(&ctx.db, per_page);
    let total = paginator.num_items_and_pages().await?.number_of_items;
    let items = paginator.fetch_page(page - 1).await?;

    Ok(Json(Page {
        items: items.into_iter().map({{ pascal_singular }}Dto::from).collect(),
        total: total as i64,
        page: page as i64,
        per_page: per_page as i64,
    }))
}

#[debug_handler]
async fn get_one(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
) -> Result<Response> {
    let item = Entity::find_by_id(id).one(&ctx.db).await?;
    match item {
        Some(model) => Ok(Json({{ pascal_singular }}Dto::from(model)).into_response()),
        None => Ok(not_found("{{ snake_singular }} not found")),
    }
}

#[debug_handler]
async fn create(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<Create{{ pascal_singular }}>,
) -> Result<Response> {
    let item = ActiveModel {
        {% for f in fields -%}
        {{ f.field_name }}: Set({{ f.set_expr }}),
        {% endfor -%}
        ..Default::default()
    };
    let item = item.insert(&ctx.db).await?;

    Ok((StatusCode::CREATED, Json({{ pascal_singular }}Dto::from(item))).into_response())
}

#[debug_handler]
async fn update(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
    Json(params): Json<Update{{ pascal_singular }}>,
) -> Result<Response> {
    let Some(model) = Entity::find_by_id(id).one(&ctx.db).await? else {
        return Ok(not_found("{{ snake_singular }} not found"));
    };

    let mut item = model.into_active_model();
    {% for f in fields -%}
    item.{{ f.field_name }} = Set({{ f.set_expr }});
    {% endfor -%}
    let item = item.update(&ctx.db).await?;

    Ok(Json({{ pascal_singular }}Dto::from(item)).into_response())
}

#[debug_handler]
async fn remove(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
) -> Result<Response> {
    let Some(model) = Entity::find_by_id(id).one(&ctx.db).await? else {
        return Ok(not_found("{{ snake_singular }} not found"));
    };

    model.into_active_model().delete(&ctx.db).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/{{ snake_plural }}")
        .add("/", get(list))
        .add("/", post(create))
        .add("/{id}", get(get_one))
        .add("/{id}", put(update))
        .add("/{id}", delete(remove))
}
