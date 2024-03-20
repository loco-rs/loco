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

#[cfg(test)]
mod tests {

    use dockertest_server::{
        servers::database::redis::{RedisServer, RedisServerConfig},
        Test,
    };
    use serial_test::serial;
    use sidekiq::redis_rs::AsyncCommands;

    use super::*;
    use crate::{boot, environment::Environment};

    #[test]
    #[serial]
    fn test_ping() {
        let mut test = Test::new();

        let config = RedisServerConfig::builder().port(9898).build().unwrap();
        test.register(config);

        test.run(|instance| async move {
            let redis: RedisServer = instance.server();

            let mut config = Environment::Test.load().unwrap();
            config.redis.as_mut().unwrap().uri = redis.external_url();
            let pool = boot::connect_redis(&config).await.unwrap();

            assert!(ping(&pool).await.is_ok());

            config.redis.as_mut().unwrap().uri = "redis://127.1.1.1".to_string();
            let pool = boot::connect_redis(&config).await.unwrap();

            assert!(ping(&pool).await.is_err());
        });
    }

    #[test]
    #[serial]
    fn can_converge() {
        let mut test = Test::new();

        let config = RedisServerConfig::builder().port(9897).build().unwrap();
        test.register(config);

        test.run(|instance| async move {
            let redis: RedisServer = instance.server();

            let mut config = Environment::Test.load().unwrap();
            config.redis.as_mut().unwrap().uri = redis.external_url();

            let pool = boot::connect_redis(&config).await.unwrap();

            let mut conn = pool.get().await.unwrap();

            let set_val = "test-app";
            // setting value into redis
            assert!(conn
                .unnamespaced_borrow_mut()
                .set::<&str, &str, String>("loco", set_val)
                .await
                .is_ok());

            // make sure we can read the value

            let res: bool = conn.unnamespaced_borrow_mut().exists("loco").await.unwrap();
            assert!(res);

            // run converge function when dangerously_flush is false, the value should be
            // exists
            config.redis.as_mut().unwrap().dangerously_flush = false;
            assert!(converge(&pool, &config.redis).await.is_ok());

            let res: bool = conn.unnamespaced_borrow_mut().exists("loco").await.unwrap();
            assert!(res);

            // now run converge command when dangerously_flush is true, expecting that the
            // value will be deleted
            config.redis.as_mut().unwrap().dangerously_flush = true;
            assert!(converge(&pool, &config.redis).await.is_ok());
            let res: bool = conn.unnamespaced_borrow_mut().exists("loco").await.unwrap();
            assert!(!res);
        });
    }

    #[test]
    #[serial]
    fn test_connection() {
        let mut test = Test::new();

        let config = RedisServerConfig::builder().port(9898).build().unwrap();
        test.register(config);

        test.run(|instance| async move {
            let redis: RedisServer = instance.server();

            let client = redis::Client::open(redis.external_url().as_str()).unwrap();
            let mut con = client.get_connection().unwrap();
            let res: String = redis::cmd("PING").query(&mut con).unwrap();
            assert_eq!(res, "PONG");
        });
    }
}
