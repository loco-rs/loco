use serde::{Deserialize, Serialize};

use crate::logger;

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
