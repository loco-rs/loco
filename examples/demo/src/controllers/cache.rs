use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CacheResponse {
    value: Option<String>,
}

async fn get_cache(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(CacheResponse {
        value: ctx.cache.get("value").await.unwrap(),
    })
}
async fn insert(State(ctx): State<AppContext>) -> Result<Response> {
    ctx.cache.insert("value", "loco cache value").await.unwrap();
    format::empty()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("cache")
        .add("/", get(get_cache))
        .add("/insert", post(insert))
}
