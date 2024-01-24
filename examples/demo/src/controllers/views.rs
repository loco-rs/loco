#![allow(clippy::unused_async)]
use axum::response::IntoResponse;
use loco_rs::prelude::*;
use tera::Context;

use crate::initializers::view_templates::TeraView;

pub async fn render_home(tera: TeraView) -> Result<impl IntoResponse> {
    let res = tera
        .tera
        .render("home/hello.html", &Context::new())
        .expect("templ");
    format::html(&res)
}

pub fn routes() -> Routes {
    Routes::new().prefix("views").add("/home", get(render_home))
}
