// use async_trait::async_trait;
use axum::{Extension, Router};
use loco_rs::{config, db, Result};

/// Adding Database connection to [`axum::Extension`] to the given
/// router
///
/// # Errors
///
/// When could not connect to database
pub async fn add(router: Router, db_config: config::Database) -> Result<Router> {
    let db = db::connect(&db_config).await?;
    Ok(router.layer(Extension(db)))
}
