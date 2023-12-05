//! # Configuration Management
//!
//! This module defines the configuration structures and functions to manage and
//! load configuration settings for the application.
use std::path::{Path, PathBuf};

use config::{ConfigError, File};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{environment::Environment, logger, Error, Result as AppResult};

lazy_static! {
    static ref DEFAULT_FOLDER: PathBuf = PathBuf::from("config");
}

/// Represents the main application configuration structure.
///
/// This struct encapsulates various configuration settings. The configuration
/// can be customized through YAML files for different environments.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub logger: Logger,
    pub server: Server,
    #[cfg(feature = "with-db")]
    pub database: Database,
    pub redis: Option<Redis>,
    pub auth: Option<Auth>,
    pub workers: Workers,
    pub mailer: Option<Mailer>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Logger {
    pub enable: bool,
    pub level: logger::LogLevel,
    pub format: logger::Format,
    pub override_filter: Option<String>,
}

/// Represents the worker mode configuration.
///
/// The `WorkerMode` enum specifies the worker mode, which can be one of
/// `BackgroundQueue`, `ForegroundBlocking`, or `BackgroundAsync`.
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub enum WorkerMode {
    #[default]
    /// Workers operate asynchronously in the background, processing queued
    /// tasks.
    BackgroundQueue,
    /// Workers operate in the foreground and block until tasks are completed.
    ForegroundBlocking,
    /// Workers operate asynchronously in the background, processing tasks with
    /// async capabilities.
    BackgroundAsync,
}

/// Represents the database configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Database {
    /// The URI for connecting to the database. For example:
    /// postgres://root:12341234@localhost:5432/rr_app
    pub uri: String,
    /// Enable SQLx statement logging
    pub enable_logging: bool,
    /// Minimum number of connections for a pool
    pub min_connections: u32,
    /// Maximum number of connections for a pool
    pub max_connections: u32,
    /// Set the timeout duration when acquiring a connection
    pub connect_timeout: u64,
    /// Set the idle duration before closing a connection
    pub idle_timeout: u64,
    #[serde(default)]
    /// Run migration up when application loaded
    pub auto_migrate: bool,
    #[serde(default)]
    /// Truncate database when application loaded
    pub dangerously_truncate: bool,
    #[serde(default)]
    /// Recreating schema
    pub dangerously_recreate: bool,
}

/// Represents the Redis configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Redis {
    /// The URI for connecting to the Redis server. For example:
    /// redis://127.0.0.1/
    pub uri: String,
    #[serde(default)]
    /// Flush redis when application loaded
    pub dangerously_flush: bool,
}

/// Represents the user authentication configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth {
    /// JWT authentication
    pub jwt: Option<JWT>,
}

/// Represents JWT configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JWT {
    /// The secret key For JWT token
    pub secret: String,
    /// The expiration time for authentication tokens.
    pub expiration: u64,
}

/// Represents the server configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Server {
    /// The port on which the server should listen for incoming connections.
    pub port: i32,
    /// The webserver host
    pub host: String,
    /// Middleware configurations for the server, including payload limits,
    /// logging, and error handling.
    pub middlewares: Middlewares,
}

/// Represents the workers configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Workers {
    pub mode: WorkerMode,
    pub queues: Option<Vec<String>>,
}

/// Represents the middleware configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Middlewares {
    /// Middleware that limit the payload request.
    pub limit_payload: Option<LimitPayloadMiddleware>,
    /// Middleware that improve the tracing logger and adding trace id for each
    /// request.
    pub logger: Option<EnableMiddleware>,
    /// catch any code panic and log the error.
    pub catch_panic: Option<EnableMiddleware>,
    /// Setting a global timeout for the requests
    pub timeout_request: Option<TimeoutRequestMiddleware>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeoutRequestMiddleware {
    pub enable: bool,
    // Timeout request in milliseconds
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitPayloadMiddleware {
    pub enable: bool,
    /// Body limit. for example: 5mb
    pub body_limit: String,
}

/// Represents a generic middleware configuration that can be enabled or
/// disabled.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnableMiddleware {
    pub enable: bool,
}

/// Represents the mailer configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mailer {
    pub smtp: Option<SmtpMailer>,

    #[cfg(feature = "testing")]
    #[serde(default)]
    pub stub: bool,
}

/// Represents the SMTP mailer configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmtpMailer {
    pub enable: bool,
    /// SMTP host. for example: localhost, smtp.gmail.com etc.
    pub host: String,
    /// SMTP port/
    pub port: u16,
    /// Enable TLS
    pub secure: bool,
    /// Auth SMTP server
    pub auth: Option<MailerAuth>,
}

/// Represents the authentication details for the mailer.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MailerAuth {
    pub user: String,
    pub password: String,
}

impl Server {
    #[must_use]
    pub fn full_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
impl Config {
    /// Creates a new configuration instance based on the specified environment.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when could not convert the give path to
    /// [`Config`] struct.
    ///
    /// # Example
    ///
    /// ```rust
    /// use loco_rs::{
    ///     config::Config,
    ///     environment::Environment,
    /// };
    ///
    /// #[tokio::main]
    /// async fn load(environment: &Environment) -> Config {
    ///     Config::new(environment).expect("configuration loading")
    /// }
    pub fn new(env: &Environment) -> Result<Self, ConfigError> {
        let config = Self::from_folder(env, DEFAULT_FOLDER.as_path())?;
        // TODO(review): Do we really want to print all config data to the logs? it
        // might be include sensitive data such DB user password, auth secrets etc...
        info!(name: "environment_loaded", config = ?config, environment = ?env, "environment loaded");

        Ok(config)
    }

    /// Loads configuration settings from a folder for the specified
    /// environment.
    ///
    /// # Errors
    /// Returns [`ConfigError`] when could not convert the give path to
    /// [`Config`] struct.
    ///
    /// # Example
    ///
    /// ```rust
    /// use loco_rs::{
    ///     config::Config,
    ///     environment::Environment,
    /// };
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn load(environment: &Environment) -> Config{
    ///     Config::from_folder(environment, &PathBuf::from("config")).expect("configuration loading")
    /// }
    pub fn from_folder(env: &Environment, path: &Path) -> Result<Self, ConfigError> {
        config::Config::builder()
            .add_source(
                File::with_name(&path.join(format!("{env}.yaml")).display().to_string())
                    .required(true),
            )
            .add_source(config::Environment::with_prefix("APP").separator("_"))
            .build()?
            .try_deserialize()
    }

    /// Get a reference to the JWT configuration.
    ///
    /// # Errors
    /// return an error when jwt token not configured
    pub fn get_jwt_config(&self) -> AppResult<&JWT> {
        self.auth
            .as_ref()
            .and_then(|auth| auth.jwt.as_ref())
            .map_or_else(
                || Err(Error::Any("sending email error".to_string().into())),
                Ok,
            )
    }
}
