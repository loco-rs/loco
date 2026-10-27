use serde::{Deserialize, Serialize};

use super::database::{db_connect_timeout, db_idle_timeout, db_max_conn, db_min_conn};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum QueueConfig {
    /// Redis queue
    Redis(RedisQueueConfig),
    /// Postgres queue (requires `db-postgres` feature)
    #[cfg(feature = "db-postgres")]
    Postgres(PostgresQueueConfig),
    /// Sqlite queue (requires `db-sqlite` feature)
    #[cfg(feature = "db-sqlite")]
    Sqlite(SqliteQueueConfig),
}

impl QueueConfig {
    /// Whether this queue is configured to discard all jobs on startup,
    /// whichever backend it uses.
    #[must_use]
    pub const fn dangerously_flush(&self) -> bool {
        match self {
            Self::Redis(config) => config.dangerously_flush,
            #[cfg(feature = "db-postgres")]
            Self::Postgres(config) => config.dangerously_flush,
            #[cfg(feature = "db-sqlite")]
            Self::Sqlite(config) => config.dangerously_flush,
        }
    }
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

    /// Opt-in visibility-timeout reaper. See [`ReaperConfig`].
    #[serde(default)]
    pub reaper: Option<ReaperConfig>,
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

    /// Opt-in visibility-timeout reaper. See [`ReaperConfig`].
    #[serde(default)]
    pub reaper: Option<ReaperConfig>,

    /// Opt-in deletion of finished jobs. See [`RetentionConfig`].
    #[serde(default)]
    pub retention: Option<RetentionConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg(feature = "db-sqlite")]
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

    /// Opt-in visibility-timeout reaper. See [`ReaperConfig`].
    #[serde(default)]
    pub reaper: Option<ReaperConfig>,

    /// Opt-in deletion of finished jobs. See [`RetentionConfig`].
    #[serde(default)]
    pub retention: Option<RetentionConfig>,
}

/// Configuration for an opt-in visibility-timeout reaper.
///
/// When set on a queue config, the queue provider spawns a background task
/// that periodically requeues jobs stuck in `Processing` for longer than
/// `age_minutes` (for example, because the worker that dequeued them
/// crashed before completing or failing them). It reuses the same requeue
/// logic as `cargo loco jobs requeue`.
///
/// This is entirely opt-in: leaving `reaper` unset (`None`, the default)
/// keeps existing behavior unchanged and no background task is spawned.
///
/// ```yaml
/// queue:
///   kind: Postgres
///   uri: "{{ get_env(name=\"LOCO_QUEUE_URL\", default=\"postgres://localhost:5432/loco_app\") }}"
///   # Optional: automatically requeue jobs stuck in `processing` (e.g. after a worker crash).
///   # reaper:
///   #   age_minutes: 10
///   #   interval_seconds: 60
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReaperConfig {
    /// Requeue jobs that have been in the `processing` state for longer than
    /// this many minutes.
    pub age_minutes: i64,

    /// How often, in seconds, the reaper sweeps for stale jobs.
    #[serde(default = "default_reaper_interval_seconds")]
    pub interval_seconds: u64,
}

/// Configuration for opt-in retention of finished jobs.
///
/// The Postgres and `SQLite` queues keep every finished job. When
/// `retention` is set, the queue provider spawns a background task that
/// periodically deletes jobs whose status is finished and whose
/// `updated_at` (the moment they finished) is older than the age set for
/// that status. A status left out is kept forever. `queued` and
/// `processing` jobs have no setting and are never deleted.
///
/// Off unless set, so upgrading never deletes anyone's jobs.
///
/// ```yaml
/// queue:
///   kind: Postgres
///   uri: "{{ get_env(name=\"LOCO_QUEUE_URL\", default=\"postgres://localhost:5432/loco_app\") }}"
///   # retention:
///   #   completed_days: 7
///   #   cancelled_days: 7
///   #   failed_days: 30
///   #   interval_seconds: 3600
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RetentionConfig {
    /// Delete `completed` jobs this many days after they completed.
    #[serde(default)]
    pub completed_days: Option<i64>,

    /// Delete `cancelled` jobs this many days after they were cancelled.
    #[serde(default)]
    pub cancelled_days: Option<i64>,

    /// Delete `failed` jobs this many days after they failed.
    #[serde(default)]
    pub failed_days: Option<i64>,

    /// How often, in seconds, the sweep runs.
    #[serde(default = "default_retention_interval_seconds")]
    pub interval_seconds: u64,
}

impl RetentionConfig {
    /// The finished statuses that have an age set, with that age in days.
    #[must_use]
    pub fn ages(&self) -> Vec<(crate::bgworker::JobStatus, i64)> {
        use crate::bgworker::JobStatus;
        [
            (JobStatus::Completed, self.completed_days),
            (JobStatus::Cancelled, self.cancelled_days),
            (JobStatus::Failed, self.failed_days),
        ]
        .into_iter()
        .filter_map(|(status, days)| days.map(|d| (status, d)))
        .collect()
    }
}

fn default_retention_interval_seconds() -> u64 {
    3600
}

fn pgq_poll_interval() -> u32 {
    1
}

#[cfg(feature = "db-sqlite")]
fn sqlt_poll_interval() -> u32 {
    1
}

fn num_workers() -> u32 {
    2
}

fn default_reaper_interval_seconds() -> u64 {
    60
}
