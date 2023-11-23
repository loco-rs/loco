//! # Redis Operations
//!
//! This module defines functions and operations related to the application's
//! redis interactions.
use bb8::Pool;
use sidekiq::redis_rs::cmd;

use crate::{worker::RedisConnectionManager, Result};

/// converge Redis logic
pub async fn converge(
    pool: &Pool<RedisConnectionManager>,
    config: &Option<crate::config::Redis>,
) -> Result<()> {
    if let Some(cfg) = config {
        if cfg.dangerously_flush {
            let mut conn = pool.get().await?;
            cmd("FLUSHDB")
                .query_async::<_, ()>(conn.unnamespaced_borrow_mut())
                .await?;

            return Ok(());
        }
    }

    Ok(())
}

#[cfg(feature = "with-db")]
/// Run Redis ping command
pub async fn ping(pool: &Pool<RedisConnectionManager>) -> Result<()> {
    let mut conn = pool.get().await?;
    Ok(cmd("PING")
        .query_async::<_, ()>(conn.unnamespaced_borrow_mut())
        .await?)
}
