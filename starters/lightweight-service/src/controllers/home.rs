use crate::views::home::HomeResponse;
use loco_rs::prelude::*;

async fn current() -> Result<Json<HomeResponse>> {
    format::json(HomeResponse::new("loco"))
}

pub fn routes() -> Routes {
    Routes::new().add("/", get(current))
}
