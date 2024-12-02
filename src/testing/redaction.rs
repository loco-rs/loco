use std::sync::OnceLock;

static CLEANUP_USER_MODEL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
static CLEANUP_DATE: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
static CLEANUP_MODEL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
static CLEANUP_MAIL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();

pub fn get_cleanup_user_model() -> &'static Vec<(&'static str, &'static str)> {
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

pub fn get_cleanup_date() -> &'static Vec<(&'static str, &'static str)> {
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

pub fn get_cleanup_model() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_MODEL.get_or_init(|| vec![(r"id: \d+,", "id: ID")])
}

pub fn get_cleanup_mail() -> &'static Vec<(&'static str, &'static str)> {
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
