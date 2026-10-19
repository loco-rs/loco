use std::time::Duration;

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::users;

#[derive(Debug, Deserialize, Serialize)]
pub struct UserStats {
    pub total: u64,
}

/// Total registered users, cached for a minute.
///
/// `ctx.cache` is the framework's seam, so it is shared process-wide and
/// swappable by configuration. A `OnceLock<Mutex<HashMap<..>>>` would compile,
/// lint and test clean while quietly reimplementing it — and would not survive
/// a move to a real cache backend.
#[debug_handler]
async fn users_total(State(ctx): State<AppContext>) -> Result<Response> {
    let total = ctx
        .cache
        .get_or_insert_with_expiry("stats:users:total", Duration::from_secs(60), async {
            users::Model::count_all(&ctx.db).await.map_err(Into::into)
        })
        .await?;

    format::json(UserStats { total })
}

pub fn routes() -> Routes {
    Routes::new().prefix("/api/stats").add("/users", get(users_total))
}
