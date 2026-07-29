//! initialization application logger.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt, fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer, Registry,
};

use crate::{app::Hooks, config, Error, Result};

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
const MODULE_WHITELIST: &[&str] = &[
    "loco_rs",
    "sea_orm_migration",
    "tower_http",
    "sqlx::query",
    "playground",
    "loco_gen",
];

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
///
/// # Errors
/// Fails if cannot initialize logger or set up an appender (in case the option
/// is enabled)
pub fn init<H: Hooks>(config: &config::Logger) -> Result<()> {
    let mut layers: Vec<Box<dyn Layer<Registry> + Sync + Send>> = Vec::new();

    if let Some(file_appender_config) = config.file_appender.as_ref()
        && file_appender_config.enable
    {
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
            Rotation::Daily => rolling_builder.rotation(tracing_appender::rolling::Rotation::DAILY),
            Rotation::Never => rolling_builder.rotation(tracing_appender::rolling::Rotation::NEVER),
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
            .map_err(Error::msg)?;

        let file_appender_layer = if file_appender_config.non_blocking {
            let (non_blocking_file_appender, work_guard) =
                tracing_appender::non_blocking(file_appender);
            NONBLOCKING_WORK_GUARD_KEEP
                .set(work_guard)
                .map_err(|_| Error::string("cannot lock for appender"))?;
            init_layer(
                non_blocking_file_appender,
                &file_appender_config.format,
                false,
            )
        } else {
            init_layer(file_appender, &file_appender_config.format, false)
        };
        layers.push(file_appender_layer);
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
    Ok(())
}

/// Builds the [`EnvFilter`] Loco applies to its tracing subscriber, following
/// the same precedence [`init`] uses: `RUST_LOG` wins, then `override_filter`,
/// then the built-in module whitelist at `level`.
///
/// Exposed as a building block so an application overriding
/// [`crate::app::Hooks::init_logger`] can reuse Loco's exact filter policy
/// while composing its own layers (e.g. adding `tracing-flame` or an OTLP
/// exporter) instead of re-deriving the whitelist by hand.
///
/// # Panics
///
/// Panics if the assembled filter directives are invalid. Unreachable in
/// practice: every directive is built from the fixed module whitelist and a
/// validated [`LogLevel`], so the constructed filter always parses.
#[must_use]
pub fn init_env_filter<H: Hooks>(override_filter: Option<&String>, level: &LogLevel) -> EnvFilter {
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

/// Builds a single boxed tracing [`Layer`] for `make_writer` in the given
/// [`Format`], with ANSI colouring toggled by `ansi` — the same layer [`init`]
/// installs for stdout and the file appender.
///
/// Exposed as a building block so an application overriding
/// [`crate::app::Hooks::init_logger`] can attach Loco's formatted layer to a
/// custom writer (a socket, an in-memory buffer, a second sink) without
/// reimplementing the compact/pretty/json formatting choice.
pub fn init_layer<W2>(
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

#[cfg(test)]
mod tests {
    use std::{
        env,
        io::Write,
        sync::{Arc, Mutex},
    };

    use serial_test::serial;

    use super::*;
    use crate::tests_cfg::db::AppHook;

    // A `MakeWriter` that captures everything written to it in an in-memory
    // buffer so we can inspect the *shape* of the formatted output produced
    // by `init_layer` (compact vs. pretty vs. json) without touching stdout
    // or the filesystem, and *without* installing a global subscriber.
    #[derive(Clone, Default)]
    struct BufferWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl BufferWriter {
        fn contents(&self) -> String {
            String::from_utf8(self.buf.lock().expect("lock").clone()).expect("utf8 log output")
        }
    }

    struct BufferWriterHandle(Arc<Mutex<Vec<u8>>>);

    impl Write for BufferWriterHandle {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("lock").extend_from_slice(data);
            Ok(data.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for BufferWriter {
        type Writer = BufferWriterHandle;

        fn make_writer(&'a self) -> Self::Writer {
            BufferWriterHandle(self.buf.clone())
        }
    }

    /// Builds the layer for `format`, emits a single tracing event through it
    /// with a *scoped* (non-global) default subscriber, and returns whatever
    /// got written. This never touches the process-wide global subscriber
    /// (which can only be installed once), so it's safe to call from any
    /// number of tests.
    fn capture_layer_output(format: &Format) -> String {
        let writer = BufferWriter::default();
        let layer = init_layer(writer.clone(), format, false);
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(field = "value", "hello world");
        });

        writer.contents()
    }

    #[test]
    fn test_init_layer_json_emits_json_object() {
        let output = capture_layer_output(&Format::Json);
        let trimmed = output.trim();
        assert!(
            trimmed.starts_with('{') && trimmed.ends_with('}'),
            "expected a json object, got: {output}"
        );
        assert!(output.contains("hello world"), "output: {output}");
        assert!(output.contains("\"field\":\"value\""), "output: {output}");
    }

    #[test]
    fn test_init_layer_compact_emits_single_line_non_json() {
        let output = capture_layer_output(&Format::Compact);
        let trimmed = output.trim();
        assert!(!trimmed.is_empty());
        assert_eq!(
            output.lines().count(),
            1,
            "compact format should render on a single line, got: {output}"
        );
        assert!(
            !trimmed.starts_with('{'),
            "compact format should not be json, got: {output}"
        );
        assert!(output.contains("hello world"), "output: {output}");
    }

    #[test]
    fn test_init_layer_pretty_emits_multi_line() {
        let output = capture_layer_output(&Format::Pretty);
        assert!(
            output.lines().count() > 1,
            "pretty format should render across multiple lines, got: {output}"
        );
        assert!(output.contains("hello world"), "output: {output}");
    }

    fn restore_rust_log(original: std::result::Result<String, env::VarError>) {
        match original {
            Ok(v) => {
                // SAFETY: test-local env restore; serialized via #[serial] so
                // no other test observes `RUST_LOG` concurrently.
                unsafe { env::set_var("RUST_LOG", v) };
            }
            Err(_) => {
                // SAFETY: test-local env restore; serialized via #[serial] so
                // no other test observes `RUST_LOG` concurrently.
                unsafe { env::remove_var("RUST_LOG") };
            }
        }
    }

    #[test]
    #[serial(rust_log_env)]
    fn test_init_env_filter_default_whitelist_uses_configured_level() {
        let original = env::var("RUST_LOG");
        // SAFETY: test-local env setup; serialized via #[serial] so no other
        // test observes `RUST_LOG` concurrently.
        unsafe { env::remove_var("RUST_LOG") };

        let filter = init_env_filter::<AppHook>(None, &LogLevel::Warn);
        let rendered = filter.to_string();

        for module in MODULE_WHITELIST {
            assert!(
                rendered.contains(&format!("{module}=warn")),
                "expected a `{module}=warn` directive in `{rendered}`"
            );
        }
        assert!(
            rendered.contains(&format!("{}=warn", AppHook::app_name())),
            "expected an app-name directive in `{rendered}`"
        );

        restore_rust_log(original);
    }

    #[test]
    #[serial(rust_log_env)]
    fn test_init_env_filter_override_filter_replaces_whitelist_when_no_rust_log() {
        let original = env::var("RUST_LOG");
        // SAFETY: test-local env setup; serialized via #[serial] so no other
        // test observes `RUST_LOG` concurrently.
        unsafe { env::remove_var("RUST_LOG") };

        let override_filter = "my_crate=trace".to_string();
        let filter = init_env_filter::<AppHook>(Some(&override_filter), &LogLevel::Info);
        let rendered = filter.to_string();

        assert_eq!(rendered, "my_crate=trace");
        // the whitelist should be entirely bypassed when an override is set
        assert!(!rendered.contains("loco_rs="), "rendered: {rendered}");

        restore_rust_log(original);
    }

    #[test]
    #[serial(rust_log_env)]
    fn test_init_env_filter_rust_log_takes_precedence_over_override_and_whitelist() {
        let original = env::var("RUST_LOG");
        // SAFETY: test-local env setup; serialized via #[serial] so no other
        // test observes `RUST_LOG` concurrently.
        unsafe { env::set_var("RUST_LOG", "error") };

        let override_filter = "debug".to_string();
        let filter = init_env_filter::<AppHook>(Some(&override_filter), &LogLevel::Info);
        let rendered = filter.to_string();

        assert_eq!(
            rendered, "error",
            "RUST_LOG should win over override_filter"
        );

        restore_rust_log(original);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Off.to_string(), "off");
        assert_eq!(LogLevel::Trace.to_string(), "trace");
        assert_eq!(LogLevel::Debug.to_string(), "debug");
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Error.to_string(), "error");
    }
}
