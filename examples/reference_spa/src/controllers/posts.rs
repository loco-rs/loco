#![allow(clippy::unused_async)]
use axum::http::StatusCode;
use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder};
use serde::Deserialize;

use crate::{
    dtos::{
        common::{ApiError, Page},
        posts::{CreatePost, PostDto, UpdatePost},
    },
    models::_entities::posts::{ActiveModel, Column, Entity},
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
    pub status: Option<String>,
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
    let page = params.page.max(1);
    let per_page = params.per_page.max(1);

    let mut query = Entity::find();
    if let Some(status) = &params.status {
        query = query.filter(Column::Status.eq(status.clone()));
    }
    query = query.order_by_asc(Column::Id);

    let paginator = query.paginate(&ctx.db, per_page);
    let total = paginator.num_items_and_pages().await?.number_of_items;
    let items = paginator.fetch_page(page - 1).await?;

    Ok(Json(Page {
        items: items.into_iter().map(PostDto::from).collect(),
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
        Some(model) => Ok(Json(PostDto::from(model)).into_response()),
        None => Ok(not_found("post not found")),
    }
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
