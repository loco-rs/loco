use roco_rs::prelude::*;

use crate::views::home::HomeResponse;

#[debug_handler]
async fn current() -> Result<Response> {
    format::json(HomeResponse::new("roco"))
}

pub fn routes() -> Routes {
    Routes::new().prefix("/api").add("/", get(current))
}
