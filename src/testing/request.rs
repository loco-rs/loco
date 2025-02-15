use std::net::SocketAddr;

use axum_test::{TestServer, TestServerConfig};
use tokio::net::TcpListener;

use crate::{
    app::{AppContext, Hooks},
    boot::{self, BootResult},
    config::Server,
    environment::Environment,
    Result,
};

/// The port on which the test server will run.
pub const TEST_PORT_SERVER: i32 = 5555;

/// The hostname to which the test server binds.
pub const TEST_BINDING_SERVER: &str = "localhost";

/// Constructs and returns the base URL used for the test server.
#[must_use]
pub fn get_base_url() -> String {
    format!("http://{TEST_BINDING_SERVER}:{TEST_PORT_SERVER}/")
}

/// Constructs and returns the base URL used for the test server.
#[must_use]
pub fn get_base_url_port(port: i32) -> String {
    format!("http://{TEST_BINDING_SERVER}:{port}/")
}

/// Returns a unique port number. Usually increments by 1 starting from 59126
///
/// # Panics
///
/// Will panic if binding to test server address fails or if getting the local address fails
pub async fn get_available_port() -> i32 {
    let addr = format!("{TEST_BINDING_SERVER}:0");
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    i32::from(
        listener
            .local_addr()
            .expect("Failed to get local address")
            .port(),
    )
}

/// Bootstraps test application with test environment hard coded.
///
/// # Errors
/// when could not bootstrap the test environment
///
/// # Example
///
/// The provided example demonstrates how to boot the test case with the
/// application context.
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing::prelude::*;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = boot_test::<App, Migrator>().await;
///
///     /// .....
///     assert!(false)
/// }
/// ```
pub async fn boot_test<H: Hooks>() -> Result<BootResult> {
    let config = H::load_config(&Environment::Test).await?;
    H::boot(boot::StartMode::ServerOnly, &Environment::Test, config).await
}

/// Bootstraps test application with test environment hard coded,
/// and with a unique port.
///
/// # Errors
/// when could not bootstrap the test environment
///
/// # Example
///
/// The provided example demonstrates how to boot the test case with the
/// application context, and a with a unique port.
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing::prelude::*;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let port = get_available_port().await;
///     let boot = boot_test_unique_port::<App, Migrator>(Some(port)).await;
///
///     /// .....
///     assert!(false)
/// }
pub async fn boot_test_unique_port<H: Hooks>(port: Option<i32>) -> Result<BootResult> {
    let mut config = H::load_config(&Environment::Test).await?;
    config.server = Server {
        port: port.unwrap_or(TEST_PORT_SERVER),
        binding: TEST_BINDING_SERVER.to_string(),
        ..config.server
    };
    H::boot(boot::StartMode::ServerOnly, &Environment::Test, config).await
}

#[allow(clippy::future_not_send)]
/// Initiates a test request with a provided callback.
///
///
/// # Panics
/// When could not initialize the test request.this errors can be when could not
/// initialize the test app
///
/// # Example
///
/// The provided example demonstrates how to create a test that check
/// application HTTP endpoints
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing::prelude::*;
///
///     #[tokio::test]
/// #[serial]
/// async fn can_register() {
///     request::<App, _, _>(|request, ctx| async move {
///         let response = request.post("/auth/register").json(&serde_json::json!({})).await;
///
///         with_settings!({
///             filters => cleanup_user_model()
///         }, {
///             assert_debug_snapshot!(response);
///         });
///     })
///     .await;
/// }
/// ```
#[allow(clippy::future_not_send)]
pub async fn request<H: Hooks, F, Fut>(callback: F)
where
    F: FnOnce(TestServer, AppContext) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let boot = boot_test::<H>().await.unwrap();

    let config = TestServerConfig {
        default_content_type: Some("application/json".to_string()),
        ..Default::default()
    };
    let server = TestServer::new_with_config(
        boot.router
            .unwrap()
            .into_make_service_with_connect_info::<SocketAddr>(),
        config,
    )
    .unwrap();

    callback(server, boot.app_context.clone()).await;
}
