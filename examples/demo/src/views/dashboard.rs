use loco_rs::prelude::*;
use serde_json::json;

pub fn home(v: impl ViewRenderer) -> Result<impl IntoResponse> {
    format::render().view(&v, "home/hello.html", json!(()))
}
