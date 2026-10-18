use sqlx::PgPool;
use std::time::Duration;
use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

/// Sets up an empty `PostgreSQL` database for one test.
///
/// With `LOCO_TEST_POSTGRES_URL` set to a server URL without a database path
/// (e.g. `postgres://postgres@127.0.0.1:5432`), this creates a uniquely named
/// database on that server and needs no Docker. Otherwise it starts a
/// container with [`setup_postgres_container`].
///
/// # Returns
///
/// The connection URL, and the container when one was started. Keep the
/// container alive for as long as the test uses the database.
///
/// # Panics
///
/// This function will panic if it cannot create the database or start the
/// container.
pub async fn setup_postgres_database() -> (String, Option<ContainerAsync<GenericImage>>) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    let Ok(server) = std::env::var("LOCO_TEST_POSTGRES_URL") else {
        let (pg_url, container) = setup_postgres_container().await;
        return (pg_url, Some(container));
    };
    let server = server.trim_end_matches('/');
    let name = format!(
        "loco_test_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    );
    let admin = PgPool::connect(&format!("{server}/postgres"))
        .await
        .expect("Failed to connect to LOCO_TEST_POSTGRES_URL");
    for statement in [
        format!("DROP DATABASE IF EXISTS {name}"),
        format!("CREATE DATABASE {name}"),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&admin)
            .await
            .expect("Failed to create test database");
    }
    admin.close().await;
    (format!("{server}/{name}"), None)
}

/// Sets up a `PostgreSQL` test container.
///
/// # Returns
///
/// A tuple containing the `PostgreSQL` connection URL and the container instance.
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
