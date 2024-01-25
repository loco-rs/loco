#![allow(clippy::unused_async)]
use axum::response::IntoResponse;
use loco_rs::prelude::*;

use crate::initializers::view_templates::{Engine, TemplateEngine, TeraView};

pub async fn render_home(tera: Engine<TeraView>) -> Result<impl IntoResponse> {
    let res = tera.render("home/hello.html", ()).expect("templ");
    format::html(&res)
}

pub fn routes() -> Routes {
    Routes::new().prefix("views").add("/home", get(render_home))
}
