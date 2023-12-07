use axum::{extract::State, response::Html, routing::get};
use loco_rs::{
    app::AppContext,
    controller::{format, Routes},
    Result,
};
use tera::Tera;

const HOMEPAGE_T: &str = include_str!("../views/homepage.t");

async fn index(State(ctx): State<AppContext>) -> Result<Html<String>> {
    let mut context = tera::Context::new();
    context.insert("environment", &ctx.environment.to_string());

    format::html(&Tera::one_off(HOMEPAGE_T, &context, true)?)
}

pub fn routes() -> Routes {
    Routes::new().add("/", get(index))
}
