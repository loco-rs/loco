use axum::{
    extract::State,
    routing::{get, post},
};
use rustyrails::{
    app::AppContext,
    controller::{format, Routes},
    Result,
};

async fn index(State(_ctx): State<AppContext>) -> Result<String> {
    format::text("Rustyrails")
}

pub async fn echo(req_body: String) -> String {
    req_body
}

pub fn routes() -> Routes {
    Routes::new().add("/", get(index)).add("/echo", post(echo))
}
