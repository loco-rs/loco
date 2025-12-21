use sqlx::MySqlPool;
use std::time::Duration;
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt, core::{ContainerPort, WaitFor, logs::LogSource, wait::LogWaitStrategy}, runners::AsyncRunner
};

/// Sets up a `MySQL` test container.
/// 
/// # Returns
///
/// A tuple containing the `MySQL` connection URL and the container instance.
///
/// # Panics
///
/// This function will panic if it fails to set up, start, or connect to the MySQL container.
pub async fn setup_mysql_container() -> (String, ContainerAsync<GenericImage>) {
    let mysql_image = GenericImage::new("mysql", "8")
        .with_wait_for(WaitFor::log( LogWaitStrategy::new(LogSource::StdErr,
                "ready for connections",)))
        .with_exposed_port(ContainerPort::Tcp(3306))
        .with_env_var("MYSQL_ROOT_PASSWORD", "mysql")
        .with_env_var("MYSQL_DATABASE", "loco_test")
        .with_env_var("MYSQL_USER", "loco")
        .with_env_var("MYSQL_PASSWORD", "loco");

    let container = mysql_image
        .start()
        .await
        .expect("Failed to start MySQL container");

    let host_port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("Failed to get host port");

    // Construct the URL. Note: MySQL protocol usually takes the form mysql://user:pass@host:port/db
    let mysql_url = format!("mysql://loco:loco@127.0.0.1:{host_port}/loco_test");

    // Connection retry logic (identical to your Postgres version)
    let mut connected = false;

    for attempt in 0..10 {
        // Note: Requires sqlx with 'mysql' feature for testing
        match MySqlPool::connect(&mysql_url).await {
            Ok(pool) => {
                match sqlx::query("SELECT 1").execute(&pool).await {
                    Ok(_) => {
                        connected = true;
                        break;
                    }
                    Err(_) => {
                        if attempt < 9 {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            }
            Err(_) => {
                if attempt < 9 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    assert!(
        connected,
        "Failed to connect to MySQL after 10 attempts"
    );

    (mysql_url, container)
}