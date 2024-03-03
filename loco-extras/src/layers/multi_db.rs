use std::collections::HashMap;

use axum::{Extension, Router};
use loco_rs::{config, db, Result};

/// Adding [`db::MultiDb`] as a [`axum::Extension`] to the given router
///
/// # Errors
///
/// When could not open connection to at leasts on of the given Db's
pub async fn add(router: Router, db_config: HashMap<String, config::Database>) -> Result<Router> {
    let multi_db = db::MultiDb::new(db_config).await?;
    Ok(router.layer(Extension(multi_db)))
}
