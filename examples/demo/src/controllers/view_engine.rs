#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use serde_json::json;

use crate::{initializers::hello_view_engine::HelloView, views};

/// Renders the dashboard home page
///
/// # Errors
///
/// This function will return an error if render fails
pub async fn render_home(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    views::engine::home(&v)
}

/// Hello
///
/// # Errors
///
/// This function will return an error if render fails
pub async fn render_hello(ViewEngine(v): ViewEngine<HelloView>) -> Result<Response> {
    // NOTE: v is a hello engine, which always returns 'hello', params dont matter.
    // it's a funky behavior that we use for demonstrating how easy it is
    // to build a custom view engine.
    format::render().view(&v, "foobar", ())
}

pub async fn render_simple() -> Result<Response> {
    format::render().view(
        &TeraViewInline::default(),
        "{{name}} website",
        json!({"name": "Loco"}),
    )
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("view-engine")
        .add("/home", get(render_home))
        .add("/hello", get(render_hello))
        .add("/simple", get(render_simple))
}
