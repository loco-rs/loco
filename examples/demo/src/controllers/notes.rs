#![allow(clippy::unused_async)]
use axum::{extract::State, routing::get};

use framework::{
    app::AppContext,
    controller::{format, Routes},
    errors::Error,
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

/// Benchmark function for a simple hello-world endpoint.
///
/// # Errors
///
/// Errors related to formatting the response.
pub async fn bench_hello(_req_body: String) -> Result<String> {
    format::text("hello")
}

/// Benchmark function for database operations.
///
/// This function is used to benchmark database operations by performing a simple
/// query to retrieve a user entity by ID. It utilizes the `Entity::find_by_id` method
/// provided by the `users` module.
///
/// # Errors
///
/// When db query fails
pub async fn bench_db(State(ctx): State<AppContext>) -> Result<()> {
    let _ = users::Entity::find_by_id(1).one(&ctx.db).await?;
    format::empty()
}

/// return echo message
pub async fn echo(req_body: String) -> String {
    req_body
}

/// A simple endpoint for demonstrating asynchronous tasks in a web application.
///
/// # Errors
///
/// when send welcome message fails or there is an error when preform download task
pub async fn hello(State(ctx): State<AppContext>) -> Result<String> {
    DownloadWorker::perform_later(
        &ctx,
        DownloadWorkerArgs {
            user_guid: "foo".to_string(),
        },
    )
    .await
    .map_err(|e| {
        tracing::error!(
            error = e.to_string(),
            "could not perform the download worker"
        );
        Error::Any("could not perform the download worker ".into())
    })?;

    AuthMailer::send_welcome(&ctx, "foobar").await?;

    format::text("hello")
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(hello))
        .add("/echo", get(echo))
        .add("/bench_db", get(bench_db))
        .add("/bench_hello", get(bench_hello))
}
