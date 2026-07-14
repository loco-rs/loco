//! This module contains utility functions and constants for working with
//! environment variables in the application. It centralizes the logic for
//! fetching environment variables, ensuring that keys are easily accessible
//! from a single location in the codebase.

#[cfg(feature = "with-db")]
/// The key for `PostgreSQL` database options environment variable.
pub const ROCO_POSTGRES_DB_OPTIONS: &str = "ROCO_POSTGRES_DB_OPTIONS";
#[cfg(feature = "with-db")]
pub const LOCO_POSTGRES_DB_OPTIONS: &str = "LOCO_POSTGRES_DB_OPTIONS";
/// The key for the application's environment (e.g., development, production).
pub const ROCO_ENV: &str = "ROCO_ENV";
/// Legacy application environment key retained for Loco compatibility.
///
/// [`ROCO_ENV`] takes precedence when both keys are set.
pub const LOCO_ENV: &str = "LOCO_ENV";
/// The key for the application's environment (e.g., development, production).
pub const RAILS_ENV: &str = "RAILS_ENV";
/// The key for the application's environment (e.g., development, production).
pub const NODE_ENV: &str = "NODE_ENV";
// The key for the application environment configuration
pub const ROCO_CONFIG_FOLDER: &str = "ROCO_CONFIG_FOLDER";
pub const LOCO_CONFIG_FOLDER: &str = "LOCO_CONFIG_FOLDER";
// The key for the scheduler configuration
pub const SCHEDULER_CONFIG: &str = "SCHEDULER_CONFIG";
/// The key for the data folder path
pub const ROCO_DATA_FOLDER_ENV: &str = "ROCO_DATA";
pub const LOCO_DATA_FOLDER_ENV: &str = "LOCO_DATA";

/// Fetches the value of the given environment variable.
pub fn get(key: &str) -> Result<String, std::env::VarError> {
    std::env::var(key)
}

#[allow(dead_code)]
/// Retrieves the value of the given environment variable, or returns a default value if the variable is not set.
pub fn get_or_default(key: &str, default: &str) -> String {
    get(key).unwrap_or_else(|_| default.to_string())
}
