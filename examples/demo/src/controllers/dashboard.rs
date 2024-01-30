#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

use crate::views;

/// Renders the dashboard home page
///
/// # Errors
///
/// This function will return an error if render fails
pub async fn render_home(ViewEngine(v): ViewEngine<TeraView>) -> Result<impl IntoResponse> {
    views::dashboard::home(v)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("dashboard")
        .add("/home", get(render_home))
}
