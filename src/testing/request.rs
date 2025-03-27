use std::net::SocketAddr;

use axum_test::{TestServer, TestServerConfig};

#[cfg(feature = "with-db")]
use crate::Error;

use crate::{
    app::{AppContext, Hooks},
    boot::{self, BootResult},
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

/// Configuration for making requests in the test server.
pub struct RequestConfig {
    /// Determines whether cookies should be saved for future requests.
    pub save_cookies: bool,
    /// The default content type for all requests.
    pub default_content_type: Option<String>,
    /// The default scheme to use for requests (e.g., "http" or "https").
    pub default_scheme: String,
}

impl Default for RequestConfig {
    fn default() -> Self {
        RequestConfigBuilder::new().build()
    }
}

/// Builder pattern for constructing [`RequestConfig`] instances.
pub struct RequestConfigBuilder {
    save_cookies: bool,
    default_content_type: Option<String>,
    default_scheme: String,
}

impl RequestConfigBuilder {
    /// Creates a new [`RequestConfigBuilder`] with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            save_cookies: false,
            default_content_type: Some("application/json".to_string()),
            default_scheme: "http".to_string(),
        }
    }

    /// Sets whether cookies should be saved for future requests.
    #[must_use]
    pub fn save_cookies(mut self, save: bool) -> Self {
        self.save_cookies = save;
        self
    }

    /// Sets the default content type for requests.
    #[must_use]
    pub fn default_content_type<S: Into<String>>(mut self, content_type: S) -> Self {
        self.default_content_type = Some(content_type.into());
        self
    }

    /// Sets the default scheme to use for requests.
    #[must_use]
    pub fn default_scheme<S: Into<String>>(mut self, scheme: S) -> Self {
        self.default_scheme = scheme.into();
        self
    }

    /// Builds and returns a `RequestConfig` instance.
    #[must_use]
    pub fn build(self) -> RequestConfig {
        RequestConfig {
            save_cookies: self.save_cookies,
            default_content_type: self.default_content_type,
            default_scheme: self.default_scheme,
        }
    }
}

impl Default for RequestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the From trait for automatic conversion
impl From<RequestConfig> for TestServerConfig {
    fn from(request_config: RequestConfig) -> Self {
        Self {
            default_content_type: request_config.default_content_type,
            save_cookies: request_config.save_cookies,
            ..Default::default()
        }
    }
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

#[allow(clippy::future_not_send)]
async fn request_internal<F, Fut>(callback: F, boot: &BootResult, test_server_config: RequestConfig)
where
    F: FnOnce(TestServer, AppContext) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let routes = boot.router.clone().unwrap();
    let server = TestServer::new_with_config(
        routes.into_make_service_with_connect_info::<SocketAddr>(),
        test_server_config,
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
    request_with_config::<H, F, Fut>(RequestConfig::default(), callback).await;
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
    request_config_with_create_db::<H, F, Fut>(RequestConfig::default(), callback).await;
}

/// Executes a test server request using a custom [`RequestConfig`].
///
/// This function will boot the test environment without creating a new database.
/// It takes a `config` parameter to customize request settings and a `callback`
/// function that is called with the test server and application context.
///
/// # Panics
/// When the test request cannot be initialized, such as when the test app fails to start.
///
/// # Example
/// ```rust,ignore
/// let config = RequestConfigBuilder::new().save_cookies(true).build();
/// request_with_config::<App, _, _>(config, |request, ctx| async move {
///     let response = request.get("/endpoint").await;
/// });
/// ```
pub async fn request_with_config<H: Hooks, F, Fut>(config: RequestConfig, callback: F)
where
    F: FnOnce(TestServer, AppContext) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let boot: BootResult = boot_test::<H>().await.unwrap();
    request_internal::<F, Fut>(callback, &boot, config).await;
}

/// Executes a test server request with a created database using a custom [`RequestConfig`].
///
/// This function initializes the test environment, sets up a fresh database, and then runs
/// the provided callback function with the test server and application context.
/// The test database will be cleaned up after the test completes.
///
/// # Panics
/// When the test request cannot be initialized, such as when the test app fails to start.
///
/// # Example
/// ```rust,ignore
/// let config = RequestConfigBuilder::new().save_cookies(true).build();
/// request_config_with_create_db::<App, _, _>(config, |request, ctx| async move {
///     let response = request.get("/endpoint").await;
/// });
/// ```
#[allow(clippy::future_not_send)]
#[cfg(feature = "with-db")]
pub async fn request_config_with_create_db<H: Hooks, F, Fut>(config: RequestConfig, callback: F)
where
    F: FnOnce(TestServer, AppContext) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let boot_wrapper: BootResultWrapper = boot_test_with_create_db::<H>().await.unwrap();
    request_internal::<F, Fut>(callback, &boot_wrapper.inner, config).await;
}
