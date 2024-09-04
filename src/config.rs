//! # Configuration Management
//!
//! This module defines the configuration structures and functions to manage and
//! load configuration settings for the application.

/***
=============
CONTRIBUTORS:
=============

Here's a check list when adding configuration values:

* Add the new configuration piece
* Document each field with the appropriate rustdoc comment
* Go to `starters/`, evaluate which starter needs a configuration update, and update as needed.
  apply a YAML comment above the new field or section with explanation and possible values.

Notes:
* Configuration is feature-dependent: with and without database
* Configuration is "stage" dependent: development, test, production
* We typically provide best practice values for development and test, but by-design we do not provide default values for production

***/

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use fs_err as fs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::{
    controller::middleware::{remote_ip::RemoteIPConfig, secure_headers::SecureHeadersConfig},
    environment::Environment,
    logger, Error, Result,
};

lazy_static! {
    static ref DEFAULT_FOLDER: PathBuf = PathBuf::from("config");
}

/// Main application configuration structure.
///
/// This struct encapsulates various configuration settings. The configuration
/// can be customized through YAML files for different environments.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub logger: Logger,
    pub server: Server,
    #[cfg(feature = "with-db")]
    pub database: Database,
    pub queue: Option<Redis>,
    pub auth: Option<Auth>,
    #[serde(default)]
    pub workers: Workers,
    pub mailer: Option<Mailer>,
    pub initializers: Option<Initializers>,

    /// Custom app settings
    ///
    /// Example:
    /// ```yaml
    /// settings:
    ///   allow_list:
    ///     - google.com
    ///     - apple.com
    /// ```
    /// And then optionally deserialize it to your own `Settings` type by
    /// accessing `ctx.config.settings`.
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
}

/// Logger configuration
///
/// The Loco logging stack is built on `tracing`, using a carefuly
/// crafted stack of filters and subscribers. We filter out noise,
/// apply a log level across your app, and sort out back traces for
/// a great developer experience.
///
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// logger:
///   enable: true
///   pretty_backtrace: true
///   level: debug
///   format: compact
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Logger {
    /// Enable log write to stdout
    pub enable: bool,

    /// Enable nice display of backtraces, in development this should be on.
    /// Turn it off in performance sensitive production deployments.
    #[serde(default)]
    pub pretty_backtrace: bool,

    /// Set the logger level.
    ///
    /// * options: `trace` | `debug` | `info` | `warn` | `error`
    pub level: logger::LogLevel,

    /// Set the logger format.
    ///
    /// * options: `compact` | `pretty` | `json`
    pub format: logger::Format,

    /// Override our custom tracing filter.
    ///
    /// Set this to your own filter if you want to see traces from internal
    /// libraries. See more [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives)
    pub override_filter: Option<String>,

    /// Set this if you want to write log to file
    pub file_appender: Option<LoggerFileAppender>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LoggerFileAppender {
    /// Enable logger file appender
    pub enable: bool,

    /// Enable write log to file non-blocking
    #[serde(default)]
    pub non_blocking: bool,

    /// Set the logger file appender level.
    ///
    /// * options: `trace` | `debug` | `info` | `warn` | `error`
    pub level: logger::LogLevel,

    /// Set the logger file appender format.
    ///
    /// * options: `compact` | `pretty` | `json`
    pub format: logger::Format,

    /// Set the logger file appender rotation.
    pub rotation: logger::Rotation,

    /// Set the logger file appender dir
    ///
    /// default is `./logs`
    pub dir: Option<String>,

    /// Set log filename prefix
    pub filename_prefix: Option<String>,

    /// Set log filename suffix
    pub filename_suffix: Option<String>,

    /// Set the logger file appender keep max log files.
    pub max_log_files: usize,
}

/// Database configuration
///
/// Configures the [SeaORM](https://www.sea-ql.org/SeaORM/) connection and pool, as well as Loco's additional DB
/// management utils such as `auto_migrate`, `truncate` and `recreate`.
///
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// database:
///   uri: {{ get_env(name="DATABASE_URL", default="...") }}
///   enable_logging: true
///   connect_timeout: 500
///   idle_timeout: 500
///   min_connections: 1
///   max_connections: 1
///   auto_migrate: true
///   dangerously_truncate: false
///   dangerously_recreate: false
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Database {
    /// The URI for connecting to the database. For example:
    /// * Postgres: `postgres://root:12341234@localhost:5432/myapp_development`
    /// * Sqlite: `sqlite://db.sqlite?mode=rwc`
    pub uri: String,

    /// Enable `SQLx` statement logging
    pub enable_logging: bool,

    /// Minimum number of connections for a pool
    pub min_connections: u32,

    /// Maximum number of connections for a pool
    pub max_connections: u32,

    /// Set the timeout duration when acquiring a connection
    pub connect_timeout: u64,

    /// Set the idle duration before closing a connection
    pub idle_timeout: u64,

    /// Set the timeout for acquiring a connection
    pub acquire_timeout: Option<u64>,

    /// Run migration up when application loads. It is recommended to turn it on
    /// in development. In production keep it off, and explicitly migrate your
    /// database every time you need.
    #[serde(default)]
    pub auto_migrate: bool,

    /// Truncate database when application loads. It will delete data from your
    /// tables. Commonly used in `test`.
    #[serde(default)]
    pub dangerously_truncate: bool,

    /// Recreate schema when application loads. Use it when you want to reset
    /// your database *and* structure (drop), this also deletes all of the data.
    /// Useful when you're just sketching out your project and trying out
    /// various things in development.
    #[serde(default)]
    pub dangerously_recreate: bool,
}

/// Redis Configuration
///
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// redis:
///   uri: redis://127.0.0.1/
///   dangerously_flush: false
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Redis {
    /// The URI for connecting to the Redis server. For example:
    /// <redis://127.0.0.1/>
    pub uri: String,
    #[serde(default)]
    /// Flush redis when application loaded. Useful for `test`.
    pub dangerously_flush: bool,
}

/// User authentication configuration.
///
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// auth:
///   jwt:
///     secret: <your secret>
///     expiration: 604800 # 7 days
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth {
    /// JWT authentication config
    pub jwt: Option<JWT>,
}

/// JWT configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JWT {
    /// The location where JWT tokens are expected to be found during
    /// authentication.
    pub location: Option<JWTLocation>,
    /// The secret key For JWT token
    pub secret: String,
    /// The expiration time for authentication tokens
    pub expiration: u64,
}

/// Defines the authentication mechanism for middleware.
///
/// This enum represents various ways to authenticate using JSON Web Tokens
/// (JWT) within middleware.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "from")]
pub enum JWTLocation {
    /// Authenticate using a Bearer token.
    Bearer,
    /// Authenticate using a token passed as a query parameter.
    Query { name: String },
    /// Authenticate using a token stored in a cookie.
    Cookie { name: String },
}

/// Server configuration structure.
///
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// server:
///   port: {{ get_env(name="NODE_PORT", default=5150) }}
///   host: http://localhost
///   middlewares:
///     limit_payload:
///       enable: true
///       body_limit: 5mb
///     logger:
///       enable: true
///     catch_panic:
///       enable: true
///     timeout_request:
///       enable: true
///       timeout: 5000
///     compression:
///       enable: true
///     cors:
///       enable: true
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Server {
    /// The address on which the server should listen on for incoming
    /// connections.
    #[serde(default = "default_binding")]
    pub binding: String,
    /// The port on which the server should listen for incoming connections.
    pub port: i32,
    /// The webserver host
    pub host: String,
    /// Identify via the `Server` header
    pub ident: Option<String>,
    /// Middleware configurations for the server, including payload limits,
    /// logging, and error handling.
    pub middlewares: Middlewares,
}

fn default_binding() -> String {
    "localhost".to_string()
}

impl Server {
    #[must_use]
    pub fn full_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
/// Background worker configuration
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// workers:
///   mode: BackgroundQueue
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Workers {
    /// Toggle between different worker modes
    pub mode: WorkerMode,
    /// Custom queue names declaration. Required if you set up a dedicated
    /// worker against a dedicated queue.
    pub queues: Option<Vec<String>>,
}

/// Worker mode configuration
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub enum WorkerMode {
    /// Workers operate asynchronously in the background, processing queued
    /// tasks. **Requires a Redis connection**.
    #[default]
    BackgroundQueue,
    /// Workers operate in the foreground in the same process and block until
    /// tasks are completed.
    ForegroundBlocking,
    /// Workers operate asynchronously in the background, processing tasks with
    /// async capabilities in the same process.
    BackgroundAsync,
}

/// Server middleware configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Middlewares {
    /// Middleware that enable compression for the response.
    pub compression: Option<EnableMiddleware>,
    /// Middleware that enable etag cache headers.
    pub etag: Option<EnableMiddleware>,
    /// Middleware that limit the payload request.
    pub limit_payload: Option<LimitPayloadMiddleware>,
    /// Middleware that improve the tracing logger and adding trace id for each
    /// request.
    pub logger: Option<EnableMiddleware>,
    /// catch any code panic and log the error.
    pub catch_panic: Option<EnableMiddleware>,
    /// Setting a global timeout for the requests
    pub timeout_request: Option<TimeoutRequestMiddleware>,
    /// Setting cors configuration
    pub cors: Option<CorsMiddleware>,
    /// Serving static assets
    #[serde(rename = "static")]
    pub static_assets: Option<StaticAssetsMiddleware>,
    /// Sets a set of secure headers
    pub secure_headers: Option<SecureHeadersConfig>,
    /// Calculates a remote IP based on `X-Forwarded-For` when behind a proxy
    pub remote_ip: Option<RemoteIPConfig>,
    /// Configure fallback behavior when hitting a missing URL
    pub fallback: Option<FallbackConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FallbackConfig {
    /// By default when enabled, returns a prebaked 404 not found page optimized
    /// for development. For production set something else (see fields below)
    pub enable: bool,
    /// For the unlikely reason to return something different than `404`, you
    /// can set it here
    pub code: Option<u16>,
    /// Returns content from a file pointed to by this field with a `404` status
    /// code.
    pub file: Option<String>,
    /// Returns a "404 not found" with a single message string. This sets the
    /// message.
    pub not_found: Option<String>,
}
/// Static asset middleware configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticAssetsMiddleware {
    pub enable: bool,
    /// Check that assets must exist on disk
    pub must_exist: bool,
    /// Assets location
    pub folder: FolderAssetsMiddleware,
    /// Fallback page for a case when no asset exists (404). Useful for SPA
    /// (single page app) where routes are virtual.
    pub fallback: String,
    /// Enable `precompressed_gzip`
    #[serde(default = "bool::default")]
    pub precompressed: bool,
}

/// Asset folder config.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FolderAssetsMiddleware {
    /// Uri for the assets
    pub uri: String,
    /// Path for the assets
    pub path: String,
}

/// CORS middleware configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CorsMiddleware {
    pub enable: bool,
    /// Allow origins
    pub allow_origins: Option<Vec<String>>,
    /// Allow headers
    pub allow_headers: Option<Vec<String>>,
    /// Allow methods
    pub allow_methods: Option<Vec<String>>,
    /// Max age
    pub max_age: Option<u64>,
}

/// Timeout middleware configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeoutRequestMiddleware {
    pub enable: bool,
    // Timeout request in milliseconds
    pub timeout: u64,
}

/// Limit payload size middleware configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitPayloadMiddleware {
    pub enable: bool,
    /// Body limit. for example: 5mb
    pub body_limit: String,
}

/// A generic middleware configuration that can be enabled or
/// disabled.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnableMiddleware {
    pub enable: bool,
}

/// Mailer configuration
///
/// Example (development), to capture mails with something like [mailcrab](https://github.com/tweedegolf/mailcrab):
/// ```yaml
/// # config/development.yaml
/// mailer:
///   smtp:
///     enable: true
///     host: localhost
///     port: 1025
///     secure: false
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mailer {
    pub smtp: Option<SmtpMailer>,

    #[serde(default)]
    pub stub: bool,
}

/// Initializers configuration
///
/// Example (development): To configure settings for oauth2 or custom view
/// engine
/// ```yaml
/// # config/development.yaml
/// initializers:
///  oauth2:
///   authorization_code: # Authorization code grant type
///     - client_identifier: google # Identifier for the `OAuth2` provider.
///       Replace 'google' with your provider's name if different, must be
///       unique within the oauth2 config. ... # other fields
pub type Initializers = BTreeMap<String, serde_json::Value>;

/// SMTP mailer configuration structure.
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

/// Authentication details for the mailer
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MailerAuth {
    /// User
    pub user: String,
    /// Password
    pub password: String,
}

impl Config {
    /// Creates a new configuration instance based on the specified environment.
    ///
    /// # Errors
    ///
    /// Returns error when could not convert the give path to
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
    pub fn new(env: &Environment) -> Result<Self> {
        let config = Self::from_folder(env, DEFAULT_FOLDER.as_path())?;
        Ok(config)
    }

    /// Loads configuration settings from a folder for the specified
    /// environment.
    ///
    /// # Errors
    /// Returns error when could not convert the give path to
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
    pub fn from_folder(env: &Environment, path: &Path) -> Result<Self> {
        // by order of precedence
        let files = [
            path.join(format!("{env}.local.yaml")),
            path.join(format!("{env}.yaml")),
        ];

        let selected_path = files
            .iter()
            .find(|p| p.exists())
            .ok_or_else(|| Error::Message("no configuration file found".to_string()))?;

        info!(selected_path =? selected_path, "loading environment from");

        let content = fs::read_to_string(selected_path)?;
        let rendered = crate::tera::render_string(&content, &json!({}))?;

        serde_yaml::from_str(&rendered)
            .map_err(|err| Error::YAMLFile(err, selected_path.to_string_lossy().to_string()))
    }

    /// Get a reference to the JWT configuration.
    ///
    /// # Errors
    /// return an error when jwt token not configured
    pub fn get_jwt_config(&self) -> Result<&JWT> {
        self.auth
            .as_ref()
            .and_then(|auth| auth.jwt.as_ref())
            .map_or_else(
                || Err(Error::Any("no JWT config found".to_string().into())),
                Ok,
            )
    }
}
