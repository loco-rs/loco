to: src/controllers/{{ snake_plural }}.rs
skip_exists: true
message: "Controller `{{ pascal_singular }}` was added successfully.{% if auth %} Its routes require a JWT — re-run with `--no-auth` to generate public routes.{% endif %}"
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
use sea_orm::QueryOrder;

use crate::{
    dtos::{
        common::{ApiError, Page},
        {{ snake_plural }}::{Create{{ pascal_singular }}, {{ pascal_singular }}Dto, Update{{ pascal_singular }}},
    },
    models::_entities::{{ snake_plural }}::{ActiveModel, Column, Entity},
};

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
{% if auth %}    _auth: auth::JWT,
{% endif %}    State(ctx): State<AppContext>,
    Query(pagination): Query<query::PaginationQuery>,
) -> Result<Json<Page<{{ pascal_singular }}Dto>>> {
    let res = query::paginate(
        &ctx.db,
        Entity::find().order_by_asc(Column::Id),
        None,
        &pagination,
    )
    .await?;

    Ok(Json(Page::from_query(res)))
}

#[debug_handler]
async fn get_one(
{% if auth %}    _auth: auth::JWT,
{% endif %}    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
) -> Result<Response> {
    let Some(model) = Entity::find_by_id(id).one(&ctx.db).await? else {
        return Ok(not_found("{{ snake_singular }} not found"));
    };
    Ok(Json({{ pascal_singular }}Dto::from(model)).into_response())
}

#[debug_handler]
async fn create(
{% if auth %}    _auth: auth::JWT,
{% endif %}    State(ctx): State<AppContext>,
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
{% if auth %}    _auth: auth::JWT,
{% endif %}    State(ctx): State<AppContext>,
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
{% if auth %}    _auth: auth::JWT,
{% endif %}    State(ctx): State<AppContext>,
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
