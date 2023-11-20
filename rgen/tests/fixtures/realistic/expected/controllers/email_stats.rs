#![allow(clippy::unused_async)]
use axum::{extract::State, routing::get};
use rustyrails::{
    app::AppContext,
    controller::{format, Routes},
    Result,
};

pub async fn echo(req_body: String) -> String {
    req_body
}

pub async fn hello(State(ctx): State<AppContext>) -> Result<String> {
    // do something with context (database, etc)
    format::text("hello")
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("email_stats")
        .add("/", get(hello))
        .add("/echo", get(echo))
}