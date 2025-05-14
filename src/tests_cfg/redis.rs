use redis::Client;
use std::time::Duration;
use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage,
};

/// Sets up a Redis test container.
///
/// # Returns
///
/// A tuple containing the Redis URL and the container instance.
///
/// # Panics
///
/// This function will panic if it fails to set up, start, or connect to the Redis container.
pub async fn setup_redis_container() -> (String, ContainerAsync<GenericImage>) {
    let redis_image = GenericImage::new("redis", "7")
        .with_exposed_port(ContainerPort::Tcp(6379))
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

    let container = redis_image
        .start()
        .await
        .expect("Failed to start Redis container");

    let host_port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("Failed to get host port");

    let redis_url = format!("redis://127.0.0.1:{host_port}");

    // Try to ping Redis up to 10 times with 1 second interval
    let client = Client::open(redis_url.clone()).expect("Failed to create Redis client");

    let mut connected = false;
    for attempt in 0..10 {
        match client.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                // Try to ping
                if redis::cmd("PING")
                    .query_async::<()>(&mut conn)
                    .await
                    .is_ok()
                {
                    // Successfully pinged Redis
                    connected = true;
                    break;
                } else if attempt < 9 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
            Err(_) => {
                if attempt < 9 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    // Panic if we couldn't connect after all attempts
    assert!(connected, "Failed to connect to Redis after 10 attempts");

    (redis_url, container)
}
