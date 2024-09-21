//! # Test Utilities Module
//!
//! This module provides utility functions and constants for easy testing
//! purposes, including cleaning up data patterns and bootstrapping the
//! application for testing.

use axum_test::{TestServer, TestServerConfig};
use lazy_static::lazy_static;
#[cfg(feature = "with-db")]
use sea_orm::DatabaseConnection;

use crate::{
    app::{AppContext, AppContextTrait, Hooks},
    boot::{self, BootResult},
    environment::Environment,
    Result,
};

// Lazy-static constants for data cleanup patterns
lazy_static! {
    /// Constants for cleaning up user model data, replacing certain patterns with placeholders.
    pub static ref CLEANUP_USER_MODEL: Vec<(&'static str, &'static str)> = vec![
        (
            r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
            "PID"
        ),
        (r"password: (.*{60}),", "password: \"PASSWORD\","),
        (r"([A-Za-z0-9-_]*\.[A-Za-z0-9-_]*\.[A-Za-z0-9-_]*)","TOKEN")
    ];

    /// Constants for cleaning up date data, replacing date-time patterns with placeholders.
    pub static ref CLEANUP_DATE: Vec<(&'static str, &'static str)> =
        vec![
            (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?\+\d{2}:\d{2}", "DATE"), // with tz
            (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d+", "DATE"),
            (r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})", "DATE")
            ];

    /// Constants for cleaning up generals model data, replacing IDs with placeholders.
    pub static ref CLEANUP_MODEL: Vec<(&'static str, &'static str)> = vec![(r"id: \d+,", "id: ID")];
    pub static ref CLEANUP_MAIL: Vec<(&'static str, &'static str)> = vec![
            (r"[0-9A-Za-z]+{40}", "IDENTIFIER"),
            (r"\w+, \d{1,2} \w+ \d{4} \d{2}:\d{2}:\d{2} [+-]\d{4}", "DATE"),
            (r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})","RANDOM_ID"),

            // also handles line break in text-format emails, where they break into a new line and then use '=' as continuation symbol.
            // #6c23875d-3523-4805-8527-f2=\r\n82d3aa7514
            // #6c23875d-3523-4805-8527-f282d3aa75=\r\n14 (note postfix after '=' can be short)
            (r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4})-[0-9a-fA-F]{4}-.*[0-9a-fA-F]{2}", "RANDOM_ID")
        ];
}

/// Combines cleanup filters from various categories (user model, date, and
/// model) into one list. This is used for data cleaning and pattern
/// replacement.
///
/// # Example
///
/// The provided example demonstrates how to efficiently clean up a user model.
/// This process is particularly valuable when you need to capture a snapshot of
/// user model data that includes dynamic elements such as incrementing IDs,
/// automatically generated PIDs, creation/update timestamps, and similar
/// attributes.
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = testing::boot_test::<App, Migrator>().await;
///
///     // Create a user and save into the database.
///
///     // capture the snapshot and cleanup the data.
///     with_settings!({
///         filters => testing::cleanup_user_model()
///     }, {
///         assert_debug_snapshot!(saved_user);
///     });
/// }
/// ```
#[must_use]
pub fn cleanup_user_model() -> Vec<(&'static str, &'static str)> {
    let mut combined_filters = CLEANUP_USER_MODEL.to_vec();
    combined_filters.extend(CLEANUP_DATE.iter().copied());
    combined_filters.extend(CLEANUP_MODEL.iter().copied());
    combined_filters
}

/// Combines cleanup filters from emails  that can be dynamic
#[must_use]
pub fn cleanup_email() -> Vec<(&'static str, &'static str)> {
    let mut combined_filters = CLEANUP_MAIL.to_vec();
    combined_filters.extend(CLEANUP_DATE.iter().copied());
    combined_filters
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
/// use myapp::app::{App, AppContext};
/// use loco_rs::testing;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = testing::boot_test::<AppContext, App, Migrator>().await;
///
///     /// .....
///     assert!(false)
/// }
/// ```
pub async fn boot_test<AC: AppContextTrait, H: Hooks<AC>>() -> Result<BootResult<AC>> {
    H::boot(boot::StartMode::ServerOnly, &Environment::Test).await
}

#[cfg(feature = "with-db")]
/// Seeds data into the database.
///
///
/// # Errors
/// When seed fails
///
/// # Example
///
/// The provided example demonstrates how to boot the test case and run seed
/// data.
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = testing::boot_test::<App, Migrator>().await;
///     testing::seed::<App>(&boot.app_context.db).await.unwrap();
///
///     /// .....
///     assert!(false)
/// }
/// ```
pub async fn seed<AC: AppContextTrait, H: Hooks<AC>>(db: &DatabaseConnection) -> Result<()> {
    let path = std::path::Path::new("src/fixtures");
    H::seed(db, path).await
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
/// use loco_rs::testing;
///
///     #[tokio::test]
/// #[serial]
/// async fn can_register() {
///     testing::request::<App, _, _>(|request, ctx| async move {
///         let response = request.post("/auth/register").json(&serde_json::json!({})).await;
///
///         with_settings!({
///             filters => testing::cleanup_user_model()
///         }, {
///             assert_debug_snapshot!(response);
///         });
///     })
///     .await;
/// }
/// ```
#[allow(clippy::future_not_send)]
pub async fn request<AC: AppContextTrait, H: Hooks<AC>, F, Fut>(callback: F)
where
    F: FnOnce(TestServer, AC) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let boot = boot_test::<AC, H>().await.unwrap();

    let config = TestServerConfig::builder()
        .default_content_type("application/json")
        .build();

    let server = TestServer::new_with_config(boot.router.unwrap(), config).unwrap();

    callback(server, boot.app_context.clone()).await;
}
