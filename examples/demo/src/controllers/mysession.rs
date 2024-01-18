#![allow(clippy::unused_async)]
use axum_session::{Session, SessionNullPool};
use loco_rs::prelude::*;

pub async fn get_session(session: Session<SessionNullPool>) -> Result<()> {
    println!("{:#?}", session);
    format::empty()
}

pub fn routes() -> Routes {
    Routes::new().prefix("mysession").add("/", get(get_session))
}
