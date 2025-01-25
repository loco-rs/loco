//! This module contains utility functions and constants for working with
//! environment variables in the application. It centralizes the logic for
//! fetching environment variables, ensuring that keys are easily accessible
//! from a single location in the codebase.

#[cfg(feature = "with-db")]
/// The key for `PostgreSQL` database options environment variable.
pub const POSTGRES_DB_OPTIONS: &str = "LOCO_POSTGRES_DB_OPTIONS";
/// The key for the application's environment (e.g., development, production).
pub const LOCO_ENV: &str = "LOCO_ENV";
/// The key for the application's environment (e.g., development, production).
pub const RAILS_ENV: &str = "RAILS_ENV";
/// The key for the application's environment (e.g., development, production).
pub const NODE_ENV: &str = "NODE_ENV";
// The key for the application environment configuration
pub const CONFIG_FOLDER: &str = "LOCO_CONFIG_FOLDER";

/// Fetches the value of the given environment variable.
pub fn get(key: &str) -> Result<String, std::env::VarError> {
    std::env::var(key)
}

#[allow(dead_code)]
/// Retrieves the value of the given environment variable, or returns a default
/// value if the variable is not set.
pub fn get_or_default(key: &str, default: &str) -> String {
    get(key).unwrap_or_else(|_| default.to_string())
}
