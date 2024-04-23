use std::sync::Arc;

use axum::{body::Body, response::IntoResponse, routing::get, Extension};
use futures_util::StreamExt;
use kalosm::language::{Llama, ModelExt};
use loco_rs::controller::Routes;
use tokio::sync::RwLock;

#[allow(clippy::missing_const_for_fn)]
#[allow(clippy::unnecessary_wraps)]
fn infallible(t: String) -> Result<String, std::convert::Infallible> {
    Ok(t)
}

async fn candle_llm(Extension(m): Extension<Arc<RwLock<Llama>>>) -> impl IntoResponse {
    let prompt = "write binary search";
    println!("{prompt}");
    let result = m.write().await.stream_text(prompt).await.unwrap();
    println!("stream ready");

    Body::from_stream(result.map(infallible)) // Adding infallible
                                              // makes
                                              // the stream
                                              // return
                                              // Result<T, E>
}

pub fn routes() -> Routes {
    Routes::new().add("/candle-llm", get(candle_llm))
}
