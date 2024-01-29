#![allow(clippy::unused_async)]
use axum::response::IntoResponse;
use loco_rs::prelude::*;

use crate::{
    initializers::view_templates::{Engine, TeraView},
    views,
};

/// Renders the dashboard home page
///
/// # Errors
///
/// This function will return an error if render fails
pub async fn render_home(Engine(t): Engine<TeraView>) -> Result<impl IntoResponse> {
    views::dashboard::home(t)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("dashboard")
        .add("/home", get(render_home))
}
