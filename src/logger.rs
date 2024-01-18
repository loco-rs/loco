//! initialization application logger.
use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;
use tracing_subscriber::EnvFilter;

use crate::{app::Hooks, config};

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
// Function to initialize the logger based on the provided configuration
const MODULE_WHITELIST: &[&str] = &["loco_rs", "sea_orm_migration", "tower_http", "sqlx::query"];
///
/// Tracing filtering rules:
/// 1. if `RUST_LOG`, use that filter
/// 2. if we have a config, and in it `override_filter` use that filter (ignore
///    all else)
/// 3. take `MODULE_WHITELIST` and filter only events from these modules, use
///    `config.level` on each to filter their events
///
/// use cases:
/// 1. mostly, people will set the level and will trust *us* to decide which
///    modules to stream events from
/// 2. people who will disagree with us, will set the `override_filter`
///    permanently, or make up their own whitelist filtering (or suggest it to
///    use via PR)
/// 3. regardless of (1) and (2) operators in production, or elsewhere can
///    always use `RUST_LOG` to quickly diagnose a service
pub fn init<H: Hooks>(config: &config::Logger) {
    if !config.enable {
        return;
    }

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| {
            // user wanted a specific filter, don't care about our internal whitelist
            // or, if no override give them the default whitelisted filter (most common)
            config.override_filter.as_ref().map_or_else(
                || {
                    EnvFilter::try_new(
                        MODULE_WHITELIST
                            .iter()
                            .map(|m| format!("{}={}", m, config.level))
                            .chain(std::iter::once(format!(
                                "{}={}",
                                H::app_name(),
                                config.level
                            )))
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                },
                EnvFilter::try_new,
            )
        })
        .expect("logger initialization failed");

    let builder = tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter);

    match config.format {
        Format::Compact => builder.compact().init(),
        Format::Pretty => builder.pretty().init(),
        Format::Json => builder.json().init(),
    };
}
