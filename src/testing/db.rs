use crate::{
    app::{AppContext, Hooks},
    db, hash, Error, Result,
};
use sqlx::{Pool, Postgres};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tree_fs::TreeBuilder;

/// Seeds data into the database.
///
///
/// # Errors
/// When seed fails
///
/// # Example
///
/// The provided example demonstrates how to boot the test case and run seed
/// data.
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing::prelude::*;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = boot_test::<App, Migrator>().await;
///     seed::<App>(&boot.app_context).await.unwrap();
///
///     /// .....
///     assert!(false)
/// }
/// ```
pub async fn seed<H: Hooks>(ctx: &AppContext) -> Result<()> {
    let path = std::path::Path::new("src/fixtures");
    H::seed(ctx, path).await
}

/// Initializes a test database connection.
///
/// # Errors
/// Returns an error if could not create a new test db.
pub fn init_test_db_creation(conn_str: &str) -> Result<Box<dyn TestSupport>> {
    if conn_str.starts_with("postgres://") {
        PostgresTest::new(conn_str).map(|test| Box::new(test) as Box<dyn TestSupport>)
    } else if conn_str.starts_with("sqlite://") {
        SqliteTest::new(conn_str).map(|test| Box::new(test) as Box<dyn TestSupport>)
    } else {
        Ok(Box::new(Any::new(conn_str)))
    }
}

pub trait TestSupport: Send + Sync {
    /// Initializes the database.
    fn init_db<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
    /// Returns the connection string.
    fn get_connection_str(&self) -> &str;
    /// Cleans up the database.
    fn cleanup_db(&self);
}

pub struct PostgresTest {
    root_connection_string: String,
    connection_string: String,
    schema_name: String,
}

impl PostgresTest {
    /// Creates a new `PostgreSQL` test database.
    ///
    /// # Errors
    /// Returns an error if could not create DB schema.
    pub fn new(conn_str: &str) -> Result<Self> {
        let db_name = db::extract_db_name(conn_str)?;

        let current_timestamp = chrono::Utc::now().timestamp();
        let test_schema_name: String = hash::random_string(10).to_lowercase();
        let test_schema_name = format!("_loco_test_{test_schema_name}_{current_timestamp}");

        Ok(Self {
            root_connection_string: conn_str.replace(db_name, "postgres"),
            connection_string: conn_str.replace(db_name, &test_schema_name),
            schema_name: test_schema_name,
        })
    }
}

#[async_trait::async_trait]
impl TestSupport for PostgresTest {
    fn get_connection_str(&self) -> &str {
        &self.connection_string
    }

    fn init_db<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let pool = Pool::<Postgres>::connect(&self.root_connection_string)
                .await
                .expect("db connection should success");
            let query = format!("CREATE DATABASE {};", self.schema_name);

            sqlx::query(&query)
                .execute(&pool)
                .await
                .expect("create DB schema");
        })
    }

    fn cleanup_db(&self) {
        let connection_string = self.root_connection_string.clone();
        let table_name = self.schema_name.clone();

        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();

            rt.block_on(async {
                let pool = Pool::<Postgres>::connect(&connection_string)
                    .await
                    .expect("db connection should success");
                let query = format!("drop database if exists {table_name};");
                sqlx::query(&query)
                    .execute(&pool)
                    .await
                    .expect("Drop database");
            });
        });
    }
}

pub struct SqliteTest {
    connection_string: String,
    db_folder: PathBuf,
    _tree: tree_fs::Tree, // Keep the tree alive while the test runs
}

impl SqliteTest {
    /// Prepare new `SQLite` connection string.
    ///
    /// # Errors
    /// Returns an error if could not prepare the connection string
    pub fn new(conn_str: &str) -> Result<Self> {
        let db_name = db::extract_db_name(conn_str)?;

        let tree = TreeBuilder::default()
            .add_empty_file("test.sqlite")
            .create()
            .map_err(|err| {
                Error::string(&format!(
                    "could not create test database directory. err: {err}"
                ))
            })?;

        Ok(Self {
            connection_string: conn_str.replace(
                db_name,
                &tree.root.join("test.sqlite").display().to_string(),
            ),
            db_folder: tree.root.clone(),
            _tree: tree,
        })
    }
}

#[async_trait::async_trait]
impl TestSupport for SqliteTest {
    fn get_connection_str(&self) -> &str {
        &self.connection_string
    }
    fn init_db<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {})
    }

    fn cleanup_db(&self) {
        std::fs::remove_dir_all(&self.db_folder).expect("Could not delete sqlite test db");
    }
}

pub struct Any {
    connection_string: String,
}
impl Any {
    #[must_use]
    pub fn new(conn_str: &str) -> Self {
        Self {
            connection_string: conn_str.to_string(),
        }
    }
}

impl TestSupport for Any {
    fn get_connection_str(&self) -> &str {
        &self.connection_string
    }
    fn init_db<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {})
    }

    fn cleanup_db(&self) {}
}

#[cfg(test)]
mod tests {

    use super::*;
    use sqlx::Row;
    use std::{thread, time};

    async fn schema_exists(pool: &sqlx::PgPool, schema_name: &str) -> bool {
        let row =
            sqlx::query("SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_database  WHERE datname = $1)")
                .bind(schema_name)
                .fetch_one(pool)
                .await
                .expect("check if table exists");

        println!("schema_name: {row:#?}");
        row.get(0)
    }

    #[tokio::test]
    async fn sqlite_test_support() {
        let conn = "sqlite://test.sqlite?mode=rwc";
        let sqlite = SqliteTest::new(conn).expect("create Sqlite test support");

        sqlite.init_db().await;

        assert!(sqlite.db_folder.exists());
        sqlite.cleanup_db();
        assert!(!sqlite.db_folder.exists());
    }

    #[tokio::test]
    async fn postgres_test_support() {
        let (conn, _container) = crate::tests_cfg::postgres::setup_postgres_container().await;
        let pg: PostgresTest = PostgresTest::new(&conn).expect("create Postgres test support");

        pg.init_db().await;

        let pool = Pool::<Postgres>::connect(&conn)
            .await
            .expect("db connection should success");

        assert!(schema_exists(&pool, &pg.schema_name).await);

        pg.cleanup_db();

        thread::sleep(time::Duration::from_secs(1));
        assert!(!schema_exists(&pool, &pg.schema_name).await);
    }
}
