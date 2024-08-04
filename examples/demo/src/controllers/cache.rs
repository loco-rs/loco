use crate::models::users;
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

async fn get_or_insert(State(ctx): State<AppContext>) -> Result<Response> {
    let res = ctx
        .cache
        .get_or_insert("user", async {
            let user = users::Model::find_by_email(&ctx.db, "user1@example.com").await?;
            Ok(user.name)
        })
        .await;

    match res {
        Ok(username) => format::text(&username),
        Err(_e) => format::text("not found"),
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("cache")
        .add("/", get(get_cache))
        .add("/insert", post(insert))
        .add("/get_or_insert", get(get_or_insert))
}
