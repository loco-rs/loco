#![allow(clippy::unused_async)]
use axum::{extract::State, routing::get};

use framework::{
    app::AppContext,
    controller::{format, Routes},
    worker::AppWorker,
    Result,
};
use sea_orm::EntityTrait;

// user imports
use crate::{
    mailers::auth::AuthMailer,
    models::_entities::users,
    workers::downloader::{DownloadWorker, DownloadWorkerArgs},
};

pub async fn bench_hello(_req_body: String) -> Result<String> {
    format::text("hello")
}
pub async fn bench_db(State(ctx): State<AppContext>) -> Result<()> {
    let _ = users::Entity::find_by_id(1).one(&ctx.db).await;
    format::empty()
}
pub async fn echo(req_body: String) -> Result<String> {
    Ok(req_body)
}

pub async fn hello(State(ctx): State<AppContext>) -> Result<String> {
    DownloadWorker::perform_later(
        &ctx,
        DownloadWorkerArgs {
            user_guid: "foo".to_string(),
        },
    )
    .await
    .unwrap();

    AuthMailer::send_welcome(&ctx, "foobar").await.unwrap();

    format::text("hello")
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(hello))
        .add("/echo", get(echo))
        .add("/bench_db", get(bench_db))
        .add("/bench_hello", get(bench_hello))
}
