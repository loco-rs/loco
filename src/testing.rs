//! # Test Utilities Module
//!
//! This module provides utility functions and constants for easy testing
//! purposes, including cleaning up data patterns and bootstrapping the
//! application for testing.

use std::sync::OnceLock;

use axum_test::{TestServer, TestServerConfig};
#[cfg(feature = "with-db")]
use sea_orm::DatabaseConnection;

use crate::{
    app::{AppContext, Hooks},
    boot::{self, BootResult},
    environment::Environment,
    Result,
};

pub static CLEANUP_USER_MODEL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
pub static CLEANUP_DATE: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
pub static CLEANUP_MODEL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
pub static CLEANUP_MAIL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();

fn get_cleanup_user_model() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_USER_MODEL.get_or_init(|| {
        vec![
            (
                r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
                "PID",
            ),
            (r"password: (.*{60}),", "password: \"PASSWORD\","),
            (r"([A-Za-z0-9-_]*\.[A-Za-z0-9-_]*\.[A-Za-z0-9-_]*)", "TOKEN"),
        ]
    })
}

fn get_cleanup_date() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_DATE.get_or_init(|| {
        vec![
            (
                r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?\+\d{2}:\d{2}",
                "DATE",
            ), // with tz
            (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+", "DATE"),
            (r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})", "DATE"),
        ]
    })
}

fn get_cleanup_model() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_MODEL.get_or_init(|| vec![(r"id: \d+,", "id: ID")])
}

fn get_cleanup_mail() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_MAIL.get_or_init(|| {
        vec![
            (r"[0-9A-Za-z]+{40}", "IDENTIFIER"),
            (
                r"\w+, \d{1,2} \w+ \d{4} \d{2}:\d{2}:\d{2} [+-]\d{4}",
                "DATE",
            ),
            (
                r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
                "RANDOM_ID",
            ),
            (
                r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4})-[0-9a-fA-F]{4}-.*[0-9a-fA-F]{2}",
                "RANDOM_ID",
            ),
        ]
    })
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
    let mut combined_filters = get_cleanup_user_model().clone();
    combined_filters.extend(get_cleanup_date().iter().copied());
    combined_filters.extend(get_cleanup_model().iter().copied());
    combined_filters
}

/// Combines cleanup filters from emails  that can be dynamic
#[must_use]
pub fn cleanup_email() -> Vec<(&'static str, &'static str)> {
    let mut combined_filters = get_cleanup_mail().clone();
    combined_filters.extend(get_cleanup_date().iter().copied());
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
/// use myapp::app::App;
/// use loco_rs::testing;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = testing::boot_test::<App, Migrator>().await;
///
///     /// .....
///     assert!(false)
/// }
/// ```
pub async fn boot_test<H: Hooks>() -> Result<BootResult> {
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
pub async fn seed<H: Hooks>(db: &DatabaseConnection) -> Result<()> {
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

    let server = TestServer::new_with_config(boot.router.unwrap(), config).unwrap();

    callback(server, boot.app_context.clone()).await;
}
