use colored::Colorize;
use regex::Regex;
use semver::Version;
use std::fmt::Write;
use std::{
    collections::{BTreeMap, HashMap},
    process::Command,
    sync::OnceLock,
};

use crate::{
    bgworker,
    cargo_config::CargoConfig,
    config::{self, Config},
    depcheck, Error, Result,
};

const SEAORM_INSTALLED: &str = "SeaORM CLI is installed";
const SEAORM_NOT_INSTALLED: &str = "SeaORM CLI was not found";
const SEAORM_NOT_FIX: &str = r"To fix, run:
      $ cargo install sea-orm-cli";
const QUEUE_CONN_OK: &str = "queue connection: success";
const QUEUE_CONN_FAILED: &str = "queue connection: failed";
const QUEUE_NOT_CONFIGURED: &str = "queue not configured?";

// versions health
const MIN_SEAORMCLI_VER: &str = "1.1.0";
static MIN_DEP_VERSIONS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
static RE_CRATE_VERSION: OnceLock<Regex> = OnceLock::new();

fn get_re_crate_version() -> &'static Regex {
    RE_CRATE_VERSION.get_or_init(|| Regex::new(r#"(?m)^[^"]*"([^"]+)""#).unwrap())
}

fn get_min_dep_versions() -> &'static HashMap<&'static str, &'static str> {
    MIN_DEP_VERSIONS.get_or_init(|| {
        let mut min_vers = HashMap::new();

        min_vers.insert("tokio", "1.33.0");
        min_vers.insert("sea-orm", "1.1.0");
        min_vers.insert("validator", "0.20.0");
        min_vers.insert("axum", "0.8.1");

        min_vers
    })
}

/// Check latest crate version in crates.io
///
/// # Errors
///
/// This function will return an error if it fails
pub fn check_cratesio_version(crate_name: &str, current_version: &str) -> Result<Option<String>> {
    // Use cargo search to get the latest version
    let output = Command::new("cargo")
        .args(["search", crate_name, "--limit", "1"])
        .output()
        .map_err(|e| Error::Message(format!("Failed to run cargo search: {e}")))?;

    let output_str = String::from_utf8(output.stdout)
        .map_err(|e| Error::Message(format!("Invalid output from cargo search: {e}")))?;

    // Parse the version from cargo search output
    // Output format is: crate_name = "version"
    let latest_version = get_re_crate_version()
        .captures(&output_str)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| {
            Error::Message("Could not find version in cargo search output".to_string())
        })?;

    // Parse versions for comparison
    let current = Version::parse(current_version)
        .map_err(|e| Error::Message(format!("Invalid current version: {e}")))?;
    let latest = Version::parse(latest_version)
        .map_err(|e| Error::Message(format!("Invalid latest version: {e}")))?;

    // Compare versions
    if latest > current {
        Ok(Some(latest_version.to_string()))
    } else {
        Ok(None)
    }
}

/// Represents different resources that can be checked.
#[derive(PartialOrd, PartialEq, Eq, Ord, Debug)]
pub enum Resource {
    SeaOrmCLI,
    Database,
    Queue,
    Deps,
    PublishedLocoVersion,
}

/// Represents the status of a resource check.
#[derive(Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Ok,
    NotOk,
    NotConfigure,
}

/// Represents the result of a resource check.
#[derive(Debug)]
pub struct Check {
    /// The status of the check.
    pub status: CheckStatus,
    /// A message describing the result of the check.
    pub message: String,
    /// Additional information or instructions related to the check.
    pub description: Option<String>,
}

impl Check {
    #[must_use]
    pub fn valid(&self) -> bool {
        self.status != CheckStatus::NotOk
    }
    /// Convert to a Result type
    ///
    /// # Errors
    ///
    /// This function will return an error if Check fails
    pub fn to_result(&self) -> Result<()> {
        if self.valid() {
            Ok(())
        } else {
            Err(Error::Message(format!(
                "{} {}",
                self.message,
                self.description.clone().unwrap_or_default()
            )))
        }
    }
}

impl std::fmt::Display for Check {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let icon = match self.status {
            CheckStatus::Ok => "✅",
            CheckStatus::NotOk => "❌",
            CheckStatus::NotConfigure => "⚠️ ",
        };

        write!(
            f,
            "{} {}{}",
            icon,
            self.message,
            self.description
                .as_ref()
                .map(|d| format!("\n{d}"))
                .unwrap_or_default()
        )
    }
}

/// Runs checks for all configured resources.
/// # Errors
/// Error when one of the checks fail
pub async fn run_all(config: &Config, production: bool) -> Result<BTreeMap<Resource, Check>> {
    let mut checks = BTreeMap::from(
        #[cfg(feature = "with-db")]
        [(Resource::Database, check_db(&config.database).await)],
        #[cfg(not(feature = "with-db"))]
        [],
    );

    if config.workers.mode == config::WorkerMode::BackgroundQueue {
        checks.insert(Resource::Queue, check_queue(config).await);
    }

    if !production {
        checks.insert(Resource::Deps, check_deps()?);
        checks.insert(Resource::SeaOrmCLI, check_seaorm_cli()?);
        checks.insert(
            Resource::PublishedLocoVersion,
            check_published_loco_version()?,
        );
    }

    Ok(checks)
}

/// Checks "blessed" / major dependencies in a Loco app Cargo.toml, and
/// recommend to update.
/// Only if a dep exists, we check it against a min version
/// # Errors
/// Returns error if fails
pub fn check_deps() -> Result<Check> {
    let cargolock = CargoConfig::lock_from_current_dir()?;

    let crate_statuses =
        depcheck::check_crate_versions(&cargolock, get_min_dep_versions().clone())?;
    let mut report = String::new();
    let _ = write!(report, "Dependencies");
    let mut all_ok = true;

    for status in &crate_statuses {
        if let depcheck::VersionStatus::Invalid {
            version,
            min_version,
        } = &status.status
        {
            let _ = writeln!(
                report,
                "     {}: version {} does not meet minimum version {}",
                status.crate_name.yellow(),
                version.red(),
                min_version.green()
            );

            all_ok = false;
        }
    }
    Ok(Check {
        status: if all_ok {
            CheckStatus::Ok
        } else {
            CheckStatus::NotOk
        },
        message: report,
        description: None,
    })
}

/// Checks the database connection.
#[cfg(feature = "with-db")]
pub async fn check_db(config: &crate::config::Database) -> Check {
    let db_connection_failed = "DB connection: fails";
    let db_connection_success = "DB connection: success";
    match crate::db::connect(config).await {
        Ok(conn) => match conn.ping().await {
            Ok(()) => match crate::db::verify_access(&conn).await {
                Ok(()) => Check {
                    status: CheckStatus::Ok,
                    message: db_connection_success.to_string(),
                    description: None,
                },
                Err(err) => Check {
                    status: CheckStatus::NotOk,
                    message: db_connection_failed.to_string(),
                    description: Some(err.to_string()),
                },
            },
            Err(err) => Check {
                status: CheckStatus::NotOk,
                message: db_connection_failed.to_string(),
                description: Some(err.to_string()),
            },
        },
        Err(err) => Check {
            status: CheckStatus::NotOk,
            message: db_connection_failed.to_string(),
            description: Some(err.to_string()),
        },
    }
}

/// Checks the Redis connection.
pub async fn check_queue(config: &Config) -> Check {
    if let Ok(Some(queue)) = bgworker::create_queue_provider(config).await {
        match queue.ping().await {
            Ok(()) => Check {
                status: CheckStatus::Ok,
                message: format!("{}: {}", queue.describe(), QUEUE_CONN_OK),
                description: None,
            },
            Err(err) => Check {
                status: CheckStatus::NotOk,
                message: format!("{}: {}", queue.describe(), QUEUE_CONN_FAILED),
                description: Some(err.to_string()),
            },
        }
    } else {
        Check {
            status: CheckStatus::NotConfigure,
            message: QUEUE_NOT_CONFIGURED.to_string(),
            description: None,
        }
    }
}

/// Checks the presence and version of `SeaORM` CLI.
/// # Panics
/// On illegal regex
/// # Errors
/// Fails when cannot check version
pub fn check_seaorm_cli() -> Result<Check> {
    match Command::new("sea-orm-cli").arg("--version").output() {
        Ok(out) => {
            let input = String::from_utf8_lossy(&out.stdout);
            // Extract the version from the input string
            let re = Regex::new(r"(\d+\.\d+\.\d+)").unwrap();

            let version_str = re
                .captures(&input)
                .and_then(|caps| caps.get(0))
                .map(|m| m.as_str())
                .ok_or("SeaORM CLI version not found")
                .map_err(Box::from)?;

            // Parse the extracted version using semver
            let version = Version::parse(version_str).map_err(Box::from)?;

            // Parse the minimum version for comparison
            let min_version = Version::parse(MIN_SEAORMCLI_VER).map_err(Box::from)?;

            // Compare the extracted version with the minimum version
            if version >= min_version {
                Ok(Check {
                    status: CheckStatus::Ok,
                    message: SEAORM_INSTALLED.to_string(),
                    description: None,
                })
            } else {
                Ok(Check {
                    status: CheckStatus::NotOk,
                    message: format!(
                        "SeaORM CLI minimal version is `{min_version}` (you have `{version}`). \
                         Run `cargo install sea-orm-cli` to update."
                    ),
                    description: Some(SEAORM_NOT_FIX.to_string()),
                })
            }
        }
        Err(_) => Ok(Check {
            status: CheckStatus::NotOk,
            message: SEAORM_NOT_INSTALLED.to_string(),
            description: Some(SEAORM_NOT_FIX.to_string()),
        }),
    }
}

/// Check for the latest Loco version
///
/// # Errors
///
/// This function will return an error if it fails
pub fn check_published_loco_version() -> Result<Check> {
    let compiled_version = env!("CARGO_PKG_VERSION");
    match check_cratesio_version("loco-rs", compiled_version) {
        Ok(Some(v)) => Ok(Check {
            status: CheckStatus::NotOk,
            message: format!("Loco version: `{compiled_version}`, latest version: `{v}`"),
            description: Some("It is recommended to upgrade your main Loco version.".to_string()),
        }),
        Ok(None) => Ok(Check {
            status: CheckStatus::Ok,
            message: "Loco version: latest".to_string(),
            description: None,
        }),
        Err(e) => Ok(Check {
            status: CheckStatus::NotOk,
            message: format!("Checking Loco version failed: {e}"),
            description: None,
        }),
    }
}
