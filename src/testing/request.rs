use std::net::SocketAddr;

use axum_test::{TestServer, TestServerConfig};
use tokio::net::TcpListener;

#[cfg(feature = "with-db")]
use crate::Error;

use crate::{
    app::{AppContext, Hooks},
    boot::{self, BootResult},
    config::Server,
    environment::Environment,
    Result,
};
#[cfg(feature = "with-db")]
use std::ops::Deref;

#[cfg(feature = "with-db")]
pub struct BootResultWrapper {
    inner: BootResult,
    test_db: Box<dyn super::db::TestSupport>,
}

#[cfg(feature = "with-db")]
impl BootResultWrapper {
    #[must_use]
    pub fn new(boot: BootResult, test_db: Box<dyn super::db::TestSupport>) -> Self {
        Self {
            inner: boot,
            test_db,
        }
    }
}

#[cfg(feature = "with-db")]
impl Deref for BootResultWrapper {
    type Target = BootResult;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "with-db")]
impl Drop for BootResultWrapper {
    fn drop(&mut self) {
        self.test_db.cleanup_db();
    }
}

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
/// }
/// ```
///
/// # Errors
/// when could not bootstrap the test environment
pub async fn boot_test<H: Hooks>() -> Result<BootResult> {
    let config = H::load_config(&Environment::Test).await?;
    let boot = H::boot(boot::StartMode::ServerOnly, &Environment::Test, config).await?;
    Ok(boot)
}

/// Bootstraps the test application with a test environment and creates a new database.
///
/// This function initializes the test environment and sets up a fresh database for testing.
/// The test database will be used during the test, and it will be cleaned up once the test completes.
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing::prelude::*;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = boot_test_with_create_db::<App, Migrator>().await;
/// }
/// ```
///
/// # Errors
/// when could not bootstrap the test environment
#[cfg(feature = "with-db")]
pub async fn boot_test_with_create_db<H: Hooks>() -> Result<BootResultWrapper> {
    let mut config = H::load_config(&Environment::Test).await?;
    let test_db = super::db::init_test_db_creation(&config.database.uri)?;
    test_db.init_db().await;
    config.database.uri = test_db.get_connection_str().to_string();
    let boot = match H::boot(boot::StartMode::ServerOnly, &Environment::Test, config).await {
        Ok(boot) => boot,
        Err(err) => {
            test_db.cleanup_db();
            return Err(Error::string(&err.to_string()));
        }
    };

    Ok(BootResultWrapper::new(boot, test_db))
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
async fn request_internal<F, Fut>(callback: F, boot: &BootResult)
where
    F: FnOnce(TestServer, AppContext) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let config = TestServerConfig {
        default_content_type: Some("application/json".to_string()),
        ..Default::default()
    };

    let routes = boot.router.clone().unwrap();
    let server = TestServer::new_with_config(
        routes.into_make_service_with_connect_info::<SocketAddr>(),
        config,
    )
    .unwrap();

    callback(server, boot.app_context.clone()).await;
}

/// Executes a test server request using the provided callback and the default boot process.
///
/// This function will boot the test environment without creating a new database.
/// It takes a `callback` function that is called with the test server and application context.
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
/// #[tokio::test]
/// #[serial]
/// async fn can_register() {
///     request::<App, _, _>(|request, ctx| async move {
///         let response = request.post("/auth/register").json(&serde_json::json!({})).await;
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
    let boot: BootResult = boot_test::<H>().await.unwrap();
    request_internal::<F, Fut>(callback, &boot).await;
}

/// Executes a test server request with a created database using the provided callback.
///
/// This function will boot the test environment and create a new database for the test.
/// It takes a `callback` function that is called with the test server and application context.
///
/// ```rust,ignore
/// use myapp::app::App;
///
/// #[tokio::test]
/// async fn can_register() {
///     request_with_create_db::<App, _, _>(|request, ctx| async move {
///         let response = request.post("/auth/register").json(&serde_json::json!({})).await;
///     })
///     .await;
/// }
/// ```
///
/// # Panics
/// When could not initialize the test request.this errors can be when could not
/// initialize the test app
#[allow(clippy::future_not_send)]
#[cfg(feature = "with-db")]
pub async fn request_with_create_db<H: Hooks, F, Fut>(callback: F)
where
    F: FnOnce(TestServer, AppContext) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let boot_wrapper: BootResultWrapper = boot_test_with_create_db::<H>().await.unwrap();
    request_internal::<F, Fut>(callback, &boot_wrapper.inner).await;
}
