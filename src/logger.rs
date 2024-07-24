//! initialization application logger.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer, Registry};

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

// Define an enumeration for log file appender rotation
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub enum Rotation {
    #[serde(rename = "minutely")]
    Minutely,
    #[serde(rename = "hourly")]
    #[default]
    Hourly,
    #[serde(rename = "daily")]
    Daily,
    #[serde(rename = "never")]
    Never,
}

// Implement Display trait for LogLevel to enable pretty printing
impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_variant_name(self).expect("only enum supported").fmt(f)
    }
}

// Function to initialize the logger based on the provided configuration
const MODULE_WHITELIST: &[&str] = &["loco_rs", "sea_orm_migration", "tower_http", "sqlx::query"];

// Keep nonblocking file appender work guard
static NONBLOCKING_WORK_GUARD_KEEP: OnceLock<WorkerGuard> = OnceLock::new();

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
    let mut layers: Vec<Box<dyn Layer<Registry> + Sync + Send>> = Vec::new();

    if let Some(file_appender_config) = config.file_appender.as_ref() {
        if file_appender_config.enable {
            let dir = file_appender_config
                .dir
                .as_ref()
                .map_or_else(|| "./logs".to_string(), ToString::to_string);

            let mut rolling_builder = tracing_appender::rolling::Builder::default()
                .max_log_files(file_appender_config.max_log_files);

            rolling_builder = match file_appender_config.rotation {
                Rotation::Minutely => {
                    rolling_builder.rotation(tracing_appender::rolling::Rotation::MINUTELY)
                }
                Rotation::Hourly => {
                    rolling_builder.rotation(tracing_appender::rolling::Rotation::HOURLY)
                }
                Rotation::Daily => {
                    rolling_builder.rotation(tracing_appender::rolling::Rotation::DAILY)
                }
                Rotation::Never => {
                    rolling_builder.rotation(tracing_appender::rolling::Rotation::NEVER)
                }
            };

            let file_appender = rolling_builder
                .filename_prefix(
                    file_appender_config
                        .filename_prefix
                        .as_ref()
                        .map_or_else(String::new, ToString::to_string),
                )
                .filename_suffix(
                    file_appender_config
                        .filename_suffix
                        .as_ref()
                        .map_or_else(String::new, ToString::to_string),
                )
                .build(dir)
                .expect("logger file appender initialization failed");

            let file_appender_layer = if file_appender_config.non_blocking {
                let (non_blocking_file_appender, work_guard) =
                    tracing_appender::non_blocking(file_appender);
                NONBLOCKING_WORK_GUARD_KEEP.set(work_guard).unwrap();
                init_layer(non_blocking_file_appender, &config.format, false)
            } else {
                init_layer(file_appender, &config.format, false)
            };
            layers.push(file_appender_layer);
        }
    }

    if config.enable {
        let stdout_layer = init_layer(std::io::stdout, &config.format, true);
        layers.push(stdout_layer);
    }

    if !layers.is_empty() {
        let env_filter = init_env_filter::<H>(config.override_filter.as_ref(), &config.level);
        tracing_subscriber::registry()
            .with(layers)
            .with(env_filter)
            .init();
    }
}

fn init_env_filter<H: Hooks>(override_filter: Option<&String>, level: &LogLevel) -> EnvFilter {
    EnvFilter::try_from_default_env()
        .or_else(|_| {
            // user wanted a specific filter, don't care about our internal whitelist
            // or, if no override give them the default whitelisted filter (most common)
            override_filter.map_or_else(
                || {
                    EnvFilter::try_new(
                        MODULE_WHITELIST
                            .iter()
                            .map(|m| format!("{m}={level}"))
                            .chain(std::iter::once(format!("{}={}", H::app_name(), level)))
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                },
                EnvFilter::try_new,
            )
        })
        .expect("logger initialization failed")
}

fn init_layer<W2>(
    make_writer: W2,
    format: &Format,
    ansi: bool,
) -> Box<dyn Layer<Registry> + Sync + Send>
where
    W2: for<'writer> MakeWriter<'writer> + Sync + Send + 'static,
{
    match format {
        Format::Compact => fmt::Layer::default()
            .with_ansi(ansi)
            .with_writer(make_writer)
            .compact()
            .boxed(),
        Format::Pretty => fmt::Layer::default()
            .with_ansi(ansi)
            .with_writer(make_writer)
            .pretty()
            .boxed(),
        Format::Json => fmt::Layer::default()
            .with_ansi(ansi)
            .with_writer(make_writer)
            .json()
            .boxed(),
    }
}
