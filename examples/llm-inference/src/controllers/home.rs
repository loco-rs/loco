use axum::routing::get;
use loco_rs::{
    controller::{format, Routes},
    Result,
};

use crate::llm::model::infer;

async fn index() -> Result<String> {
    let out = infer("what is 'Loco'?").await?;
    format::text(&out)
}

pub fn routes() -> Routes {
    Routes::new().add("/", get(index))
}
