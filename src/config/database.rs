use serde::{Deserialize, Serialize};

/// Database configuration
///
/// Configures the [SeaORM](https://www.sea-ql.org/SeaORM/) connection and pool, as well as Loco's additional DB
/// management utils such as `auto_migrate`, `truncate` and `recreate`.
///
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// database:
///   uri: {{ get_env(name="DATABASE_URL", default="...") }}
///   enable_logging: true
///   connect_timeout: 500
///   idle_timeout: 500
///   min_connections: 1
///   max_connections: 1
///   auto_migrate: true
///   dangerously_truncate: false
///   dangerously_recreate: false
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Database {
    /// The URI for connecting to the database. For example:
    /// * Postgres: `postgres://root:12341234@localhost:5432/myapp_development`
    /// * Sqlite: `sqlite://db.sqlite?mode=rwc`
    pub uri: String,

    /// Enable `SQLx` statement logging
    pub enable_logging: bool,

    /// Minimum number of connections for a pool
    pub min_connections: u32,

    /// Maximum number of connections for a pool
    pub max_connections: u32,

    /// Set the timeout duration when acquiring a connection
    pub connect_timeout: u64,

    /// Set the idle duration before closing a connection
    pub idle_timeout: u64,

    /// Set the timeout for acquiring a connection
    pub acquire_timeout: Option<u64>,

    /// Run migration up when application loads. It is recommended to turn it on
    /// in development. In production keep it off, and explicitly migrate your
    /// database every time you need.
    #[serde(default)]
    pub auto_migrate: bool,

    /// Truncate database when application loads. It will delete data from your
    /// tables. Commonly used in `test`.
    #[serde(default)]
    pub dangerously_truncate: bool,

    /// Recreate schema when application loads. Use it when you want to reset
    /// your database *and* structure (drop), this also deletes all of the data.
    /// Useful when you're just sketching out your project and trying out
    /// various things in development.
    #[serde(default)]
    pub dangerously_recreate: bool,

    // Execute query after initializing the DB
    /// for e.g. this can be used to confiure PRAGMAs for `SQLite` where you can pass all values as a string.
    /// Default values are:
    ///
    /// PRAGMA `foreign_keys` = ON;
    ///
    /// PRAGMA `journal_mode` = WAL;
    ///
    /// PRAGMA `synchronous` = NORMAL;
    ///
    /// PRAGMA `mmap_size` = 134217728;
    ///
    /// PRAGMA `journal_size_limit` = 67108864;
    ///
    /// PRAGMA `cache_size` = 2000;
    ///
    /// PRAGMA `busy_timeout` = 5000;
    pub run_on_start: Option<String>,
}

pub(crate) fn db_min_conn() -> u32 {
    1
}

pub(crate) fn db_max_conn() -> u32 {
    20
}

pub(crate) fn db_connect_timeout() -> u64 {
    500
}

pub(crate) fn db_idle_timeout() -> u64 {
    500
}
