use std::{collections::BTreeMap, process::Command};

use crate::{
    bgworker,
    config::{self, Config, Database},
    db, Error, Result,
};

const SEAORM_INSTALLED: &str = "SeaORM CLI is installed";
const SEAORM_NOT_INSTALLED: &str = "SeaORM CLI was not found";
const SEAORM_NOT_FIX: &str = r"To fix, run:
      $ cargo install sea-orm-cli";
const DB_CONNECTION_FAILED: &str = "DB connection: fails";
const DB_CONNECTION_SUCCESS: &str = "DB connection: success";
const QUEUE_CONN_OK: &str = "queue connection: success";
const QUEUE_CONN_FAILED: &str = "queue connection: failed";
const QUEUE_NOT_CONFIGURED: &str = "queue not configured?";

/// Represents different resources that can be checked.
#[derive(PartialOrd, PartialEq, Eq, Ord)]
pub enum Resource {
    SeaOrmCLI,
    Database,
    Redis,
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
pub async fn run_all(config: &Config) -> BTreeMap<Resource, Check> {
    let mut checks = BTreeMap::from([
        (Resource::SeaOrmCLI, check_seaorm_cli()),
        (Resource::Database, check_db(&config.database).await),
    ]);

    if config.workers.mode == config::WorkerMode::BackgroundQueue {
        checks.insert(Resource::Redis, check_queue(config).await);
    }

    checks
}

/// Checks the database connection.
pub async fn check_db(config: &Database) -> Check {
    match db::connect(config).await {
        Ok(conn) => match conn.ping().await {
            Ok(()) => match db::verify_access(&conn).await {
                Ok(()) => Check {
                    status: CheckStatus::Ok,
                    message: DB_CONNECTION_SUCCESS.to_string(),
                    description: None,
                },
                Err(err) => Check {
                    status: CheckStatus::NotOk,
                    message: DB_CONNECTION_FAILED.to_string(),
                    description: Some(err.to_string()),
                },
            },
            Err(err) => Check {
                status: CheckStatus::NotOk,
                message: DB_CONNECTION_FAILED.to_string(),
                description: Some(err.to_string()),
            },
        },
        Err(err) => Check {
            status: CheckStatus::NotOk,
            message: DB_CONNECTION_FAILED.to_string(),
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
#[must_use]
pub fn check_seaorm_cli() -> Check {
    match Command::new("sea-orm-cli").arg("--version").output() {
        Ok(_) => Check {
            status: CheckStatus::Ok,
            message: SEAORM_INSTALLED.to_string(),
            description: None,
        },
        Err(_) => Check {
            status: CheckStatus::NotOk,
            message: SEAORM_NOT_INSTALLED.to_string(),
            description: Some(SEAORM_NOT_FIX.to_string()),
        },
    }
}
