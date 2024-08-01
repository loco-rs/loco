use loco_rs::prelude::*;
use serde_json::json;

/// Home view
///
/// # Errors
///
/// This function will return an error if render fails
pub fn home(v: &impl ViewRenderer) -> Result<Response> {
    format::render().view(v, "home/hello.html", json!({}))
}
