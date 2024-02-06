#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

pub async fn echo(req_body: String) -> String {
    req_body
}

pub async fn hello(State(_ctx): State<AppContext>) -> Result<String> {
    // do something with context (database, etc)
    format::text("hello")
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("oauth2")
        .add("/", get(hello))
        .add("/echo", post(echo))
}
