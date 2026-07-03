use serde::{Deserialize, Serialize};

use super::database::{db_connect_timeout, db_idle_timeout, db_max_conn, db_min_conn};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum QueueConfig {
    /// Redis queue
    Redis(RedisQueueConfig),
    /// Postgres queue
    Postgres(PostgresQueueConfig),
    /// Sqlite queue
    Sqlite(SqliteQueueConfig),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisQueueConfig {
    pub uri: String,
    #[serde(default)]
    pub dangerously_flush: bool,

    /// Custom queue names declaration. Useful to model priority queues.
    /// First queue in list is more important.
    pub queues: Option<Vec<String>>,

    #[serde(default = "num_workers")]
    pub num_workers: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresQueueConfig {
    pub uri: String,

    #[serde(default)]
    pub dangerously_flush: bool,

    #[serde(default)]
    pub enable_logging: bool,

    #[serde(default = "db_max_conn")]
    pub max_connections: u32,

    #[serde(default = "db_min_conn")]
    pub min_connections: u32,

    #[serde(default = "db_connect_timeout")]
    pub connect_timeout: u64,

    #[serde(default = "db_idle_timeout")]
    pub idle_timeout: u64,

    #[serde(default = "pgq_poll_interval")]
    pub poll_interval_sec: u32,

    #[serde(default = "num_workers")]
    pub num_workers: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqliteQueueConfig {
    pub uri: String,

    #[serde(default)]
    pub dangerously_flush: bool,

    #[serde(default)]
    pub enable_logging: bool,

    #[serde(default = "db_max_conn")]
    pub max_connections: u32,

    #[serde(default = "db_min_conn")]
    pub min_connections: u32,

    #[serde(default = "db_connect_timeout")]
    pub connect_timeout: u64,

    #[serde(default = "db_idle_timeout")]
    pub idle_timeout: u64,

    #[serde(default = "sqlt_poll_interval")]
    pub poll_interval_sec: u32,

    #[serde(default = "num_workers")]
    pub num_workers: u32,
}

fn pgq_poll_interval() -> u32 {
    1
}

fn sqlt_poll_interval() -> u32 {
    1
}

fn num_workers() -> u32 {
    2
}
