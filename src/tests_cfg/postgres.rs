use sqlx::PgPool;
use std::time::Duration;
use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

/// Sets up a PostgreSQL test container.
///
/// # Returns
///
/// A tuple containing the PostgreSQL connection URL and the container instance.
///
/// # Panics
///
/// This function will panic if it fails to set up, start, or connect to the PostgreSQL container.
pub async fn setup_postgres_container() -> (String, ContainerAsync<GenericImage>) {
    let pg_image = GenericImage::new("postgres", "15")
        .with_wait_for(WaitFor::message_on_stdout(
            "database system is ready to accept connections",
        ))
        .with_exposed_port(ContainerPort::Tcp(5432))
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "postgres");

    let container = pg_image
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get host port");

    let pg_url = format!("postgres://postgres:postgres@127.0.0.1:{host_port}/postgres");

    // Try to connect to PostgreSQL up to 10 times with 1 second interval
    let mut connected = false;

    for attempt in 0..10 {
        match PgPool::connect(&pg_url).await {
            Ok(pool) => {
                // Try to ping with a simple query
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
        "Failed to connect to PostgreSQL after 10 attempts"
    );

    (pg_url, container)
}
