#![allow(clippy::unused_async)]
use axum::response::IntoResponse;
use loco_rs::{controller::views::TemplateEngine, prelude::*};

pub fn home(t: impl TemplateEngine) -> Result<impl IntoResponse> {
    let res = t.render("home/hello.html", ()).expect("templ");
    format::render().html(&res)
}
