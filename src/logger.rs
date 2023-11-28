//! initialization application logger.
use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;
use tracing_subscriber::EnvFilter;

use crate::config;

// Define an enumeration for log levels
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub enum LogLevel {
    /// The "off" level.
    #[serde(rename = "off")]
    Off,
    /// The "trace" level.
    #[serde(rename = "trace")]
    Trace,
    /// The "debug" level.
    #[serde(rename = "debug")]
    Debug,
    /// The "info" level.
    #[serde(rename = "info")]
    #[default]
    Info,
    /// The "warn" level.
    #[serde(rename = "warn")]
    Warn,
    /// The "error" level.
    #[serde(rename = "error")]
    Error,
}

// Define an enumeration for log formats
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub enum Format {
    #[serde(rename = "compact")]
    #[default]
    Compact,
    #[serde(rename = "pretty")]
    Pretty,
    #[serde(rename = "json")]
    Json,
}

// Implement Display trait for LogLevel to enable pretty printing
impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_variant_name(self).expect("only enum supported").fmt(f)
    }
}
/*
when debug +
  loco_rs
  sea_orm_migration

* aliasing target?
* why isn't 'TRACE' appearing? on RUST_LOG=trace
* config filters is poweruser: use '*' as the default thing see if works
  will show sqlx as well
* document RUST_LOG

 */

// Function to initialize the logger based on the provided configuration
pub fn init(config: &config::Logger) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(config.level.to_string()))
        .map(|default_filter| {
            if let Some(filters) = &config.filters {
                default_filter.add_directive(filters.join(",").parse().expect("valid filters"))
            } else {
                default_filter
            }
        })
        .expect("logger initialization failed");

    let builder = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_line_number(true)
        .with_file(true);

    match config.format {
        Format::Compact => builder.compact().init(),
        Format::Pretty => builder.pretty().init(),
        Format::Json => builder.json().init(),
    };
}
