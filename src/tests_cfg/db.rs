use std::path::Path;

use async_trait::async_trait;
use sea_orm::Statement;
pub use sea_orm_migration::prelude::*;

use crate::{
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
    Result,
};

/// Get query result as string
///
/// Executes the SQL query and returns the first column value as a string.
///
/// # Panics
///
/// - If the database query fails.
/// - If the query returns no result row.
/// - If the value cannot be extracted from the first column as a String or i64.
pub async fn get_value(conn: &sea_orm::DatabaseConnection, query: &str) -> String {
    // Execute query and get the result row
    let row = conn
        .query_one(Statement::from_string(
            conn.get_database_backend(),
            query.to_owned(),
        ))
        .await
        .unwrap_or_else(|e| panic!("Query failed: {query}, error: {e}"))
        .expect("No result returned");

    // Get column names
    let columns = row.column_names();

    // Get first column name or empty string
    let col_name = columns.first().map_or("", |c| c.as_str());

    // Try as string or number, convert to lowercase for consistency
    row.try_get::<String>("", col_name)
        .or_else(|_| row.try_get::<i64>("", col_name).map(|v| v.to_string()))
        .unwrap_or_else(|_| panic!("Could not extract value for column: {col_name}"))
        .to_lowercase()
}

/// Creating a dummy db connection for docs
///
/// # Panics
/// Disabled the connection validation, should pass always
pub async fn dummy_connection() -> sea_orm::DatabaseConnection {
    let mut opt = sea_orm::ConnectOptions::new("sqlite::memory:");
    opt.test_before_acquire(false);

    sea_orm::Database::connect(opt).await.unwrap()
}

/// Creating a failing db connection for tests
///
/// # Panics
/// Set a non-existing database, disabled the connection pool creation and connection validation,
/// it should fail immediately when it's used.
pub async fn fail_connection() -> sea_orm::DatabaseConnection {
    let mut opt =
        sea_orm::ConnectOptions::new("postgres://loco:loco@127.0.0.1:9999/non_existent_db");
    opt.test_before_acquire(false)
        .connect_lazy(true)
        .connect_timeout(std::time::Duration::from_micros(1));

    sea_orm::Database::connect(opt).await.unwrap()
}

pub mod test_db {
    use std::fmt;

    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "loco")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub created_at: DateTime,
        pub updated_at: DateTime,
    }

    #[derive(Debug)]
    pub enum Loco {
        Table,
        Id,
        Name,
    }

    impl Iden for Loco {
        fn unquoted(&self, s: &mut dyn fmt::Write) {
            write!(
                s,
                "{}",
                match self {
                    Self::Table => "loco",
                    Self::Id => "id",
                    Self::Name => "name",
                }
            )
            .unwrap();
        }
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

#[derive(Debug)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}

#[derive(Debug)]
pub struct AppHook;
#[async_trait]
impl Hooks for AppHook {
    fn app_version() -> String {
        "test".to_string()
    }

    fn app_name() -> &'static str {
        "TEST"
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn connect_workers(_ctx: &AppContext, _q: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(super::task::Foo);
        tasks.register(super::task::ParseArgs);
    }

    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
