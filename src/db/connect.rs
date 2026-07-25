use std::{collections::HashMap, sync::OnceLock, time::Duration};

use regex::Regex;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection,
    DatabaseConnectionType, DbConn, DbErr, ExprTrait, Statement,
};
use url::Url;

use crate::{config, env_vars, errors::Error, Result as AppResult};

pub static EXTRACT_DB_NAME: OnceLock<Regex> = OnceLock::new();

fn re_extract_db_name() -> &'static Regex {
    EXTRACT_DB_NAME.get_or_init(|| {
        Regex::new(r"^.+://(?:.*?/)?([^/?#]+)(?:[?#]|$)").expect("Extract db regex is correct")
    })
}

#[derive(Default, Clone, Debug)]
pub struct MultiDb {
    pub db: HashMap<String, DatabaseConnection>,
}

impl MultiDb {
    /// Creating multiple DB connection from the given hashmap
    ///
    /// # Errors
    ///
    /// When could not create database connection
    pub async fn new(dbs_config: HashMap<String, config::Database>) -> AppResult<Self> {
        let mut multi_db = Self::default();

        for (db_name, db_config) in dbs_config {
            multi_db.db.insert(db_name, connect(&db_config).await?);
        }

        Ok(multi_db)
    }

    /// Retrieves a database connection instance based on the specified key
    /// name.
    ///
    /// # Errors
    ///
    /// Returns an [`AppResult`] indicating an error if the specified key does
    /// not correspond to a database connection in the current context.
    pub fn get(&self, name: &str) -> AppResult<&DatabaseConnection> {
        self.db
            .get(name)
            .map_or_else(|| Err(Error::Message("db not found".to_owned())), Ok)
    }
}

/// Verifies a user has access to data within its database
///
/// # Errors
///
/// This function will return an error if IO fails
#[allow(clippy::match_wildcard_for_single_variants)]
pub async fn verify_access(db: &DatabaseConnection) -> AppResult<()> {
    match db.inner {
        DatabaseConnectionType::SqlxPostgresPoolConnection(_) => {
            let res = db
                .query_all_raw(Statement::from_string(
                    DatabaseBackend::Postgres,
                    "SELECT * FROM pg_catalog.pg_tables WHERE tableowner = current_user;",
                ))
                .await?;
            if res.is_empty() {
                return Err(Error::string(
                    "current user has no access to tables in the database",
                ));
            }
        }
        DatabaseConnectionType::Disconnected => {
            return Err(Error::string("connection to database has been closed"));
        }
        _ => {}
    }
    Ok(())
}

/// Establish a connection to the database using the provided configuration
/// settings.
///
/// # Errors
///
/// Returns a [`sea_orm::DbErr`] if an error occurs during the database
/// connection establishment.
pub async fn connect(config: &config::Database) -> Result<DbConn, sea_orm::DbErr> {
    let mut opt = ConnectOptions::new(&config.uri);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(Duration::from_millis(config.connect_timeout))
        .idle_timeout(Duration::from_millis(config.idle_timeout))
        .sqlx_logging(config.enable_logging);

    if let Some(acquire_timeout) = config.acquire_timeout {
        opt.acquire_timeout(Duration::from_millis(acquire_timeout));
    }

    let db = Database::connect(opt).await?;

    match db.get_database_backend() {
        DatabaseBackend::Sqlite => {
            db.execute_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                config.run_on_start.clone().unwrap_or_else(|| {
                    "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA mmap_size = 134217728;
            PRAGMA journal_size_limit = 67108864;
            PRAGMA cache_size = 2000;
            PRAGMA busy_timeout = 5000;
            "
                    .to_string()
                }),
            ))
            .await?;
        }
        DatabaseBackend::Postgres | DatabaseBackend::MySql => {
            if let Some(run_on_start) = &config.run_on_start {
                db.execute_raw(Statement::from_string(
                    db.get_database_backend(),
                    run_on_start.clone(),
                ))
                .await?;
            }
        }
        bk => {
            return Err(DbErr::BackendNotSupported {
                db: bk.as_str(),
                ctx: "connect",
            });
        }
    }

    Ok(db)
}

/// Extracts the database name from a given connection string.
///
/// # Errors
///
/// This function returns an error if the connection string does not match the
/// expected format.
pub fn extract_db_name(conn_str: &str) -> AppResult<&str> {
    re_extract_db_name()
        .captures(conn_str)
        .and_then(|cap| cap.get(1).map(|db| db.as_str()))
        .ok_or_else(|| Error::string("could not extract db_name"))
}

/// Derives the admin/maintenance connection URI for a Postgres `db_uri` by
/// swapping only the path (database name) for `/postgres`, preserving the
/// scheme, credentials, host, port, and query params.
///
/// A naive whole-string `.replace(db_name, "/postgres")` would corrupt the
/// URI whenever the db name also appears as a substring of the host or
/// credentials (e.g. db "loco" with user "`loco_admin@loco.host/loco`").
///
/// # Errors
///
/// Returns an error if `db_uri` cannot be parsed as a URL.
fn admin_postgres_uri(db_uri: &str) -> AppResult<String> {
    let mut url = Url::parse(db_uri)
        .map_err(|err| Error::string(&format!("could not parse Postgres database URI: {err}")))?;
    url.set_path("/postgres");
    Ok(url.to_string())
}

///  Create a new database. This functionality is currently exclusive to Postgre
/// databases.
///
/// # Errors
///
/// Returns a [`sea_orm::DbErr`] if an error occurs during run migration up.
pub async fn create(db_uri: &str) -> AppResult<()> {
    if !db_uri.starts_with("postgres://") {
        return Err(DbErr::BackendNotSupported {
            db: "Unknown",
            ctx: "Only Postgres databases are supported for table creation",
        }
        .into());
    }
    let db_name = extract_db_name(db_uri).map_err(|_| {
        Error::string("The specified table name was not found in the given Postgres database URI")
    })?;

    let conn = admin_postgres_uri(db_uri)?;
    let db = Database::connect(conn).await?;

    Ok(create_postgres_database(db_name, &db).await?)
}

/// Create a Postgres database from the given db name.
///
/// To create the database with `LOCO_POSTGRES_DB_OPTIONS`
async fn create_postgres_database(
    db_name: &str,
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    let mut select = sea_orm::sea_query::Query::select();
    select
        .expr(sea_orm::sea_query::Expr::val(1))
        .from(sea_orm::sea_query::Alias::new("pg_database"))
        .and_where(
            sea_orm::sea_query::Expr::col(sea_orm::sea_query::Alias::new("datname")).eq(db_name),
        )
        .limit(1);

    if db.query_one(&select).await?.is_some() {
        tracing::info!(db_name, "database already exists");

        return Err(sea_orm::DbErr::Custom("database already exists".to_owned()));
    }

    let with_options = env_vars::get_or_default(env_vars::POSTGRES_DB_OPTIONS, "ENCODING='UTF8'");

    let query = format!("CREATE DATABASE {db_name} WITH {with_options}");
    tracing::info!(query, "creating postgres database");

    db.execute_raw(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        query,
    ))
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg::{db::get_value, postgres::setup_postgres_container};

    #[tokio::test]
    async fn test_sqlite_connect_success() {
        let (config, _tree_fs) = crate::tests_cfg::config::get_sqlite_test_config("test");

        let result = connect(&config).await;
        assert!(
            result.is_ok(),
            "Failed to connect to SQLite: {:?}",
            result.err()
        );

        let db = result.unwrap();
        assert_eq!(db.get_database_backend(), DatabaseBackend::Sqlite);
    }

    #[tokio::test]
    async fn test_postgres_connect_success() {
        let (pg_url, _container) = setup_postgres_container().await;

        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = pg_url;
        config.min_connections = 1;
        config.max_connections = 5;

        let result = connect(&config).await;
        assert!(
            result.is_ok(),
            "Failed to connect to PostgreSQL: {:?}",
            result.err()
        );

        let db = result.unwrap();
        assert_eq!(db.get_database_backend(), DatabaseBackend::Postgres);
    }

    #[tokio::test]
    async fn test_sqlite_default_run_on_start() {
        let (config, _tree_fs) = crate::tests_cfg::config::get_sqlite_test_config("test");

        let db = connect(&config).await.expect("Failed to connect to SQLite");

        let expected_pragmas = [
            ("foreign_keys", "1"),
            ("journal_mode", "wal"),
            ("synchronous", "1"),
            ("mmap_size", "134217728"),
            ("journal_size_limit", "67108864"),
            ("cache_size", "2000"),
            ("busy_timeout", "5000"),
        ];

        for (pragma, expected_value) in expected_pragmas {
            let query = format!("PRAGMA {pragma}");
            let actual_value = get_value(&db, &query).await;

            assert_eq!(
                actual_value,
                expected_value.to_lowercase(),
                "PRAGMA {pragma} value mismatch - expected '{expected_value}', got '{actual_value}'"
            );
        }
    }

    #[tokio::test]
    async fn test_sqlite_custom_run_on_start() {
        let (mut config, _tree_fs) =
            crate::tests_cfg::config::get_sqlite_test_config("test_custom");

        config.run_on_start = Some(
            "
            PRAGMA foreign_keys = OFF;
            PRAGMA journal_mode = DELETE;
            PRAGMA synchronous = OFF;
            PRAGMA cache_size = -1000;
            PRAGMA busy_timeout = 2000;
        "
            .to_string(),
        );

        let db = connect(&config).await.expect("Failed to connect to SQLite");

        let expected_pragmas = [
            ("foreign_keys", "0"),
            ("journal_mode", "delete"),
            ("synchronous", "0"),
            ("cache_size", "-1000"),
            ("busy_timeout", "2000"),
        ];

        for (pragma, expected_value) in expected_pragmas {
            let query = format!("PRAGMA {pragma}");
            let actual_value = get_value(&db, &query).await;

            assert_eq!(
                actual_value,
                expected_value.to_lowercase(),
                "PRAGMA {pragma} value mismatch - expected '{expected_value}', got '{actual_value}'"
            );
        }
    }

    #[tokio::test]
    async fn test_postgres_run_on_start() {
        let (pg_url, _container) = setup_postgres_container().await;

        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = pg_url;
        config.run_on_start = Some(
            "CREATE TABLE IF NOT EXISTS test_run_on_start (id SERIAL PRIMARY KEY, name TEXT);"
                .to_string(),
        );

        let db = connect(&config)
            .await
            .expect("Failed to connect to PostgreSQL");

        assert_eq!(db.get_database_backend(), DatabaseBackend::Postgres);

        let query = "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'test_run_on_start'";

        let value = get_value(&db, query).await;
        assert_eq!(value, "1", "The test_run_on_start table was not created");
    }

    #[cfg(test)]
    mod extract_db_name_tests {
        use super::*;
        use rstest::rstest;

        #[rstest]
        #[case("postgres://localhost:5432/dbname", "dbname")]
        #[case("postgres://username@localhost:5432/dbname", "dbname")]
        #[case("postgres://username:password@localhost:5432/dbname", "dbname")]
        #[case("postgres://localhost:5432/dbname?param1=value1", "dbname")]
        #[case(
            "postgres://username:password@localhost:5432/dbname?param1=value1",
            "dbname"
        )]
        #[case(
            "postgres://username:password@localhost:5432/dbname?param1=value1&param2=value2",
            "dbname"
        )]
        #[case("postgres://localhost/dbname", "dbname")]
        #[case("postgres://localhost/dbname?", "dbname")]
        #[case("sqlite://dbname.sqlite", "dbname.sqlite")]
        #[case("sqlite://dbname.sqlite?mode=rwc", "dbname.sqlite")]
        #[case("sqlite:///path/to/dbname.sqlite", "dbname.sqlite")]
        #[case("sqlite://./dbname.sqlite", "dbname.sqlite")]
        #[case("sqlite://./path/to/dbname.sqlite?mode=rwc", "dbname.sqlite")]
        #[case(
            "postgres://localhost:5432/db-name-with-hyphens",
            "db-name-with-hyphens"
        )]
        #[case(
            "postgres://localhost:5432/db_name_with_underscores",
            "db_name_with_underscores"
        )]
        #[case("postgres://localhost:5432/123numeric_db", "123numeric_db")]
        #[case("postgres://localhost:5432/dbname?", "dbname")]
        #[case("postgres://localhost:5432/dbname#fragment", "dbname")]
        #[case(
            "sqlite:///absolute/path/to/db file with spaces.sqlite",
            "db file with spaces.sqlite"
        )]
        #[case(
            "sqlite://./relative/path/to/db.sqlite?cache=shared&mode=rwc",
            "db.sqlite"
        )]
        #[case("postgres://localhost:5432/dbname?sslmode=require", "dbname")]
        #[case("postgres://localhost:5432/empty-p?assword", "empty-p")]
        fn test_extract_db_name(#[case] conn_str: &str, #[case] expected: &str) {
            let result = extract_db_name(conn_str);
            assert!(result.is_ok(), "Failed to extract db name from {conn_str}");
            assert_eq!(
                result.unwrap(),
                expected,
                "Extracted incorrect db name from {conn_str}"
            );
        }

        #[rstest]
        #[case("sqlite::memory:")]
        #[case("postgres://")]
        #[case("postgres:///")]
        #[case("postgres://localhost:5432/?param=value")]
        #[case("sqlite:")]
        #[case("file:dbname.sqlite")]
        #[case("://username:password@localhost:5432/dbname")]
        fn test_extract_db_name_failure(#[case] conn_str: &str) {
            let result = extract_db_name(conn_str);
            assert!(
                result.is_err(),
                "Expected error but got success for {conn_str}"
            );
        }
    }

    #[test]
    fn test_admin_postgres_uri_does_not_corrupt_when_db_name_is_substring_of_host_or_user() {
        // The db name "loco" is also a substring of the user ("loco") and the
        // host ("loco-db.host"). A naive `db_uri.replace(db_name, "/postgres")`
        // would corrupt those occurrences too. Assert only the path changes.
        let db_uri = "postgres://loco:pass@loco-db.host:5432/loco";

        let admin_uri = admin_postgres_uri(db_uri).expect("should parse and derive admin URI");

        let parsed = Url::parse(&admin_uri).expect("derived admin URI should be a valid URL");
        assert_eq!(parsed.path(), "/postgres");
        assert_eq!(parsed.username(), "loco");
        assert_eq!(parsed.password(), Some("pass"));
        assert_eq!(parsed.host_str(), Some("loco-db.host"));
        assert_eq!(parsed.port(), Some(5432));
    }

    #[test]
    fn test_admin_postgres_uri_preserves_query_params() {
        let db_uri =
            "postgres://user:pass@localhost:5432/mydb?sslmode=require&application_name=loco";

        let admin_uri = admin_postgres_uri(db_uri).expect("should parse and derive admin URI");

        let parsed = Url::parse(&admin_uri).expect("derived admin URI should be a valid URL");
        assert_eq!(parsed.path(), "/postgres");
        assert_eq!(
            parsed.query(),
            Some("sslmode=require&application_name=loco")
        );
        assert_eq!(parsed.host_str(), Some("localhost"));
    }

    #[tokio::test]
    async fn test_postgres_create_database() {
        let (pg_url, _container) = setup_postgres_container().await;

        let base_url = pg_url.split('/').take(3).collect::<Vec<&str>>().join("/");

        let test_db_name = "test_create_db";
        let create_db_url = format!("{base_url}/{test_db_name}");

        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = pg_url.clone();
        let db = connect(&config)
            .await
            .expect("Failed to connect to default database");

        let query = format!("SELECT COUNT(*) FROM pg_database WHERE datname = '{test_db_name}'");
        let count_before = get_value(&db, &query).await;
        assert_eq!(
            count_before, "0",
            "Test database '{test_db_name}' already exists"
        );

        let result = create(&create_db_url).await;
        assert!(
            result.is_ok(),
            "Failed to create PostgreSQL database: {:?}",
            result.err()
        );

        let query = format!("SELECT COUNT(*) FROM pg_database WHERE datname = '{test_db_name}'");
        let count_before = get_value(&db, &query).await;
        assert_eq!(
            count_before, "1",
            "Test database '{test_db_name}' not exists"
        );
    }
}
