use std::net::SocketAddr;

use axum_test::{TestServer, TestServerConfig};

use crate::{
    app::{AppContext, Hooks},
    boot::{self, BootResult},
    environment::Environment,
    Result,
};

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
    H::boot(boot::StartMode::ServerOnly, &Environment::Test).await
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
