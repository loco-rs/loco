#![allow(clippy::unused_async)]
use axum::http::StatusCode;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Deserialize;

use crate::{
    dtos::{
        common::{ApiError, Page},
        posts::{CreatePost, PostDto, UpdatePost},
    },
    models::_entities::posts::{ActiveModel, Column, Entity},
};

/// A resource with its own filters composes them with the framework's
/// pagination rather than restating `page`/`page_size`: `#[serde(flatten)]`
/// puts both on the same query string.
#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub status: Option<String>,
    #[serde(flatten)]
    pub pagination: query::PaginationQuery,
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
) -> Result<Json<Page<PostDto>>> {
    let mut select = Entity::find();
    if let Some(status) = &params.status {
        select = select.filter(Column::Status.eq(status.clone()));
    }

    let res = query::paginate(
        &ctx.db,
        select.order_by_asc(Column::Id),
        None,
        &params.pagination,
    )
    .await?;

    Ok(Json(Page::from_query(res)))
}

#[debug_handler]
async fn get_one(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
) -> Result<Response> {
    let Some(model) = Entity::find_by_id(id).one(&ctx.db).await? else {
        return Ok(not_found("post not found"));
    };
    Ok(Json(PostDto::from(model)).into_response())
}

#[debug_handler]
async fn create(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreatePost>,
) -> Result<Response> {
    let item = ActiveModel {
        title: Set(params.title),
        content: Set(params.content),
        status: Set(params.status.as_str().to_string()),
        price: Set(params.price),
        published_at: Set(None),
        ..Default::default()
    };
    let item = item.insert(&ctx.db).await?;

    Ok((StatusCode::CREATED, Json(PostDto::from(item))).into_response())
}

#[debug_handler]
async fn update(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
    Json(params): Json<UpdatePost>,
) -> Result<Response> {
    let Some(model) = Entity::find_by_id(id).one(&ctx.db).await? else {
        return Ok(not_found("post not found"));
    };

    let mut item = model.into_active_model();
    item.title = Set(params.title);
    item.content = Set(params.content);
    item.status = Set(params.status.as_str().to_string());
    item.price = Set(params.price);
    let item = item.update(&ctx.db).await?;

    Ok(Json(PostDto::from(item)).into_response())
}

#[debug_handler]
async fn remove(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
) -> Result<Response> {
    let Some(model) = Entity::find_by_id(id).one(&ctx.db).await? else {
        return Ok(not_found("post not found"));
    };

    model.into_active_model().delete(&ctx.db).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/posts")
        .add("/", get(list))
        .add("/", post(create))
        .add("/{id}", get(get_one))
        .add("/{id}", put(update))
        .add("/{id}", delete(remove))
}
