use serde::{Deserialize, Serialize};

use crate::controller::middleware;

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
    #[serde(default)]
    pub middlewares: middleware::Config,
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
}

/// Worker mode configuration
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
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
