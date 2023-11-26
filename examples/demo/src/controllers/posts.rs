#![allow(clippy::unused_async)]
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json,
};
use loco_rs::{
    app::AppContext,
    controller::{format, Routes},
    errors::Error,
    Result,
};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

use crate::models::_entities::notes;

/*
pub async fn add(State(ctx): State<AppContext>) -> Result<()> {}
pub async fn remove(State(ctx): State<AppContext>) -> Result<()> {}
pub async fn update(State(ctx): State<AppContext>) -> Result<()> {}
*/

pub async fn list(State(ctx): State<AppContext>) -> Result<Json<Vec<notes::Model>>> {
    let items = notes::Entity::find().all(&ctx.db).await?;
    format::json(items)
}
pub async fn remove(Path(id): Path<uuid::Uuid>, State(ctx): State<AppContext>) -> Result<()> {
    // TODO: extract "find one" -> load_one to a common function for delete, get,
    // update
    let item = notes::Entity::find()
        .filter(notes::Column::Pid.eq(id))
        .one(&ctx.db)
        .await?;
    let item = item.ok_or_else(|| Error::NotFound)?;

    // TODO: will this expose any internal error?
    item.delete(&ctx.db).await?;
    format::empty()
}

pub async fn get_one(
    Path(id): Path<uuid::Uuid>,
    State(ctx): State<AppContext>,
) -> Result<Json<notes::Model>> {
    let item = notes::Entity::find()
        .filter(notes::Column::Pid.eq(id))
        .one(&ctx.db)
        .await?;
    let item = item.ok_or_else(|| Error::NotFound)?;
    format::json(item)
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(list))
        .add("/:id", get(get_one))
        .add("/:id", delete(remove))
    // .add("/", post(add))
    // .add("/:id", post(update))
}
