//! The environment variable names Loco reads, in one place so a key is never
//! spelled out twice. Read them with [`std::env::var`].

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
// The key for the scheduler configuration
pub const SCHEDULER_CONFIG: &str = "SCHEDULER_CONFIG";
/// The key for the data folder path
pub const LOCO_DATA_FOLDER_ENV: &str = "LOCO_DATA";
