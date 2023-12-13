use crate::{
    boot,
    config::{Config, Database},
    db, redis,
};
use std::collections::BTreeMap;
use std::process::Command;

const SEAORM_INSTALLED: &str = "SeaORM CLI is installed";
const SEAORM_NOT_INSTALLED: &str = "SeaORM CLI was not found";
const SEAORM_NOT_FIX: &str = r"To fix, run:
      $ cargo install sea-orm-cli";
const DB_CONNECTION_FAILED: &str = "DB connection: fails";
const DB_CONNECTION_SUCCESS: &str = "DB connection: success";
const REDIS_CONNECTION_SUCCESS: &str = "Redis connection: success";
const REDIS_CONNECTION_FAILED: &str = "Redis connection: failed";
const REDIS_CONNECTION_NOT_CONFIGURE: &str = "redis not configure";

/// Represents different resources that can be checked.
#[derive(PartialOrd, PartialEq, Eq, Ord)]
pub enum Resource {
    SeaOrmCLI,
    Database,
    Redis,
}

/// Represents the status of a resource check.
#[derive(Debug)]
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

impl std::fmt::Display for Check {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let icon = match self.status {
            CheckStatus::Ok => "✅",
            CheckStatus::NotOk => "❌",
            CheckStatus::NotConfigure => "⚠️",
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
    BTreeMap::from([
        (Resource::SeaOrmCLI, check_seaorm_cli()),
        (Resource::Database, check_db(&config.database).await),
        (Resource::Redis, check_redis(config).await),
    ])
}

/// Checks the database connection.
async fn check_db(config: &Database) -> Check {
    match db::connect(config).await {
        Ok(conn) => match conn.ping().await {
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
    }
}

/// Checks the Redis connection.
async fn check_redis(config: &Config) -> Check {
    if let Some(conn) = boot::connect_redis(config).await {
        match redis::ping(&conn).await {
            Ok(()) => Check {
                status: CheckStatus::Ok,
                message: REDIS_CONNECTION_SUCCESS.to_string(),
                description: None,
            },
            Err(err) => Check {
                status: CheckStatus::NotOk,
                message: REDIS_CONNECTION_FAILED.to_string(),
                description: Some(err.to_string()),
            },
        }
    } else {
        Check {
            status: CheckStatus::NotConfigure,
            message: REDIS_CONNECTION_NOT_CONFIGURE.to_string(),
            description: None,
        }
    }
}

/// Checks the presence and version of `SeaORM` CLI.
fn check_seaorm_cli() -> Check {
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
