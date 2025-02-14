//! # Database Operations
//!
//! This module defines functions and operations related to the application's
//! database interactions.

use std::{
    collections::HashMap, fs, fs::File, io::Write, path::Path, sync::OnceLock, time::Duration,
};

use chrono::{DateTime, Utc};
use duct::cmd;
use regex::Regex;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseBackend,
    DatabaseConnection, DbBackend, DbConn, DbErr, EntityTrait, IntoActiveModel, Statement,
};
use sea_orm_migration::MigratorTrait;
use tracing::info;

use super::Result as AppResult;
use crate::{
    app::{AppContext, Hooks},
    config, doctor, env_vars,
    errors::Error,
};

pub static EXTRACT_DB_NAME: OnceLock<Regex> = OnceLock::new();
const IGNORED_TABLES: &[&str] = &[
    "seaql_migrations",
    "pg_loco_queue",
    "sqlt_loco_queue",
    "sqlt_loco_queue_lock",
];

fn re_extract_db_name() -> &'static Regex {
    EXTRACT_DB_NAME
        .get_or_init(|| Regex::new(r"/([^/]+?)(?:\?|$)").expect("Extract db regex is correct"))
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
    match db {
        DatabaseConnection::SqlxPostgresPoolConnection(_) => {
            let res = db
                .query_all(Statement::from_string(
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
        DatabaseConnection::Disconnected => {
            return Err(Error::string("connection to database has been closed"));
        }
        _ => {}
    }
    Ok(())
}
/// converge database logic
///
/// # Errors
///
///  an `AppResult`, which is an alias for `Result<(), AppError>`. It may
/// return an `AppError` variant representing different database operation
/// failures.
pub async fn converge<H: Hooks, M: MigratorTrait>(
    ctx: &AppContext,
    config: &config::Database,
) -> AppResult<()> {
    if config.dangerously_recreate {
        info!("recreating schema");
        reset::<M>(&ctx.db).await?;
        return Ok(());
    }

    if config.auto_migrate {
        info!("auto migrating");
        migrate::<M>(&ctx.db).await?;
    }

    if config.dangerously_truncate {
        info!("truncating tables");
        H::truncate(ctx).await?;
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

    if db.get_database_backend() == DatabaseBackend::Sqlite {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA mmap_size = 134217728;
            PRAGMA journal_size_limit = 67108864;
            PRAGMA cache_size = 2000;
            ",
        ))
        .await?;
    }

    Ok(db)
}

/// Extracts the database name from a given connection string.
///
/// # Errors
///
/// This function returns an error if the connection string does not match the expected format.
pub fn extract_db_name(conn_str: &str) -> AppResult<&str> {
    re_extract_db_name()
        .captures(conn_str)
        .and_then(|cap| cap.get(1).map(|db| db.as_str()))
        .ok_or_else(|| Error::string("could extract db_name"))
}
///  Create a new database. This functionality is currently exclusive to Postgre
/// databases.
///
/// # Errors
///
/// Returns a [`sea_orm::DbErr`] if an error occurs during run migration up.
pub async fn create(db_uri: &str) -> AppResult<()> {
    if !db_uri.starts_with("postgres://") {
        return Err(Error::string(
            "Only Postgres databases are supported for table creation",
        ));
    }
    let db_name = extract_db_name(db_uri).map_err(|_| {
        Error::string("The specified table name was not found in the given Postgres database URI")
    })?;

    let conn = extract_db_name(db_uri)?.replace(db_uri, "/postgres");
    let db = Database::connect(conn).await?;

    Ok(create_postgres_database(db_name, &db).await?)
}

/// Apply migrations to the database using the provided migrator.
///
/// # Errors
///
/// Returns a [`sea_orm::DbErr`] if an error occurs during run migration up.
pub async fn migrate<M: MigratorTrait>(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    M::up(db, None).await
}

/// Revert migrations to the database using the provided migrator.
///
/// # Errors
///
/// Returns a [`sea_orm::DbErr`] if an error occurs during run migration up.
pub async fn down<M: MigratorTrait>(
    db: &DatabaseConnection,
    steps: u32,
) -> Result<(), sea_orm::DbErr> {
    M::down(db, Some(steps)).await
}

/// Check the migration status of the database.
///
/// # Errors
///
/// Returns a [`sea_orm::DbErr`] if an error occurs during checking status
pub async fn status<M: MigratorTrait>(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    M::status(db).await
}

/// Reset the database, dropping and recreating the schema and applying
/// migrations.
///
/// # Errors
///
/// Returns a [`sea_orm::DbErr`] if an error occurs during reset databases.
pub async fn reset<M: MigratorTrait>(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    M::fresh(db).await?;
    migrate::<M>(db).await
}

use sea_orm::EntityName;
use serde_json::{json, Value};
/// Seed the database with data from a specified file.
/// Seeds open the file path and insert all file content into the DB.
///
/// The file content should be equal to the DB field parameters.
///
/// # Errors
///
/// Returns a [`AppResult`] if could not render the path content into
/// [`Vec<serde_json::Value>`] or could not inset the vector to DB.
#[allow(clippy::type_repetition_in_bounds)]
pub async fn seed<A>(db: &DatabaseConnection, path: &str) -> crate::Result<()>
where
    <<A as ActiveModelTrait>::Entity as EntityTrait>::Model: IntoActiveModel<A>,
    for<'de> <<A as ActiveModelTrait>::Entity as EntityTrait>::Model: serde::de::Deserialize<'de>,
    A: ActiveModelTrait + Send + Sync,
    sea_orm::Insert<A>: Send + Sync,
    <A as ActiveModelTrait>::Entity: EntityName,
{
    // Deserialize YAML file into a vector of JSON values
    let seed_data: Vec<Value> = serde_yaml::from_reader(File::open(path)?)?;

    // Insert each row
    for row in seed_data {
        let model = A::from_json(row)?;
        A::Entity::insert(model).exec(db).await?;
    }

    // Get the table name from the entity
    let table_name = A::Entity::default().table_name().to_string();

    // Get the database backend
    let db_backend = db.get_database_backend();

    // Reset auto-increment
    reset_autoincrement(db_backend, &table_name, db).await?;

    Ok(())
}

/// Checks if the specified table has an 'id' column.
///
/// This function checks if the specified table has an 'id' column, which is a
/// common primary key column. It supports `Postgres`, `SQLite`, and `MySQL`
/// database backends.
///
/// # Arguments
///
/// - `db`: A reference to the `DatabaseConnection`.
/// - `db_backend`: A reference to the `DatabaseBackend`.
/// - `table_name`: The name of the table to check.
///
/// # Returns
///
/// A `Result` containing a `bool` indicating whether the table has an 'id'
/// column.
async fn has_id_column(
    db: &DatabaseConnection,
    db_backend: &DatabaseBackend,
    table_name: &str,
) -> crate::Result<bool> {
    // First check if 'id' column exists
    let result = match db_backend {
        DatabaseBackend::Postgres => {
            let query = format!(
                "SELECT EXISTS (
              SELECT 1 
              FROM information_schema.columns 
              WHERE table_name = '{table_name}' 
              AND column_name = 'id'
          )"
            );
            let result = db
                .query_one(Statement::from_string(DatabaseBackend::Postgres, query))
                .await?;
            result.is_some_and(|row| row.try_get::<bool>("", "exists").unwrap_or(false))
        }
        DatabaseBackend::Sqlite => {
            let query = format!(
                "SELECT COUNT(*) as count 
          FROM pragma_table_info('{table_name}') 
          WHERE name = 'id'"
            );
            let result = db
                .query_one(Statement::from_string(DatabaseBackend::Sqlite, query))
                .await?;
            result.is_some_and(|row| row.try_get::<i32>("", "count").unwrap_or(0) > 0)
        }
        DatabaseBackend::MySql => {
            return Err(Error::Message(
                "Unsupported database backend: MySQL".to_string(),
            ))
        }
    };

    Ok(result)
}

/// Checks whether the specified table has an auto-increment 'id' column.
///
/// # Returns
///
/// A `Result` containing a `bool` indicating whether the table has an
/// auto-increment 'id' column.
async fn is_auto_increment(
    db: &DatabaseConnection,
    db_backend: &DatabaseBackend,
    table_name: &str,
) -> crate::Result<bool> {
    let result = match db_backend {
        DatabaseBackend::Postgres => {
            let query = format!(
                "SELECT pg_get_serial_sequence('{table_name}', 'id') IS NOT NULL as is_serial"
            );
            let result = db
                .query_one(Statement::from_string(DatabaseBackend::Postgres, query))
                .await?;
            result.is_some_and(|row| row.try_get::<bool>("", "is_serial").unwrap_or(false))
        }
        DatabaseBackend::Sqlite => {
            let query =
                format!("SELECT sql FROM sqlite_master WHERE type='table' AND name='{table_name}'");
            let result = db
                .query_one(Statement::from_string(DatabaseBackend::Sqlite, query))
                .await?;
            result.is_some_and(|row| {
                row.try_get::<String>("", "sql")
                    .is_ok_and(|sql| sql.to_lowercase().contains("autoincrement"))
            })
        }
        DatabaseBackend::MySql => {
            return Err(Error::Message(
                "Unsupported database backend: MySQL".to_string(),
            ))
        }
    };
    Ok(result)
}

/// Function to reset auto-increment
/// # Errors
/// Returns error if it fails
pub async fn reset_autoincrement(
    db_backend: DatabaseBackend,
    table_name: &str,
    db: &DatabaseConnection,
) -> crate::Result<()> {
    // Check if 'id' column exists
    let has_id_column = has_id_column(db, &db_backend, table_name).await?;
    if !has_id_column {
        return Ok(());
    }
    // Check if 'id' column is auto-increment
    let is_auto_increment = is_auto_increment(db, &db_backend, table_name).await?;
    if !is_auto_increment {
        return Ok(());
    }

    match db_backend {
        DatabaseBackend::Postgres => {
            let query_str = format!(
                "SELECT setval(pg_get_serial_sequence('{table_name}', 'id'), COALESCE(MAX(id), 0) \
                 + 1, false) FROM {table_name}"
            );
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                &query_str,
                vec![],
            ))
            .await?;
        }
        DatabaseBackend::Sqlite => {
            let query_str = format!(
                "UPDATE sqlite_sequence SET seq = (SELECT MAX(id) FROM {table_name}) WHERE name = \
                 '{table_name}'"
            );
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                &query_str,
                vec![],
            ))
            .await?;
        }
        DatabaseBackend::MySql => {
            return Err(Error::Message(
                "Unsupported database backend: MySQL".to_string(),
            ))
        }
    }
    Ok(())
}
/// Generate entity model.
/// This function using sea-orm-cli.
///
/// # Errors
///
/// Returns a [`AppResult`] if an error occurs during generate model entity.
pub async fn entities<M: MigratorTrait>(ctx: &AppContext) -> AppResult<String> {
    doctor::check_seaorm_cli()?.to_result()?;
    doctor::check_db(&ctx.config.database).await.to_result()?;

    let out = cmd!(
        "sea-orm-cli",
        "generate",
        "entity",
        "--with-serde",
        "both",
        "--output-dir",
        "src/models/_entities",
        "--database-url",
        &ctx.config.database.uri,
        "--ignore-tables",
        IGNORED_TABLES.join(","),
    )
    .stderr_to_stdout()
    .run()
    .map_err(|err| {
        Error::Message(format!(
            "failed to generate entity using sea-orm-cli binary. error details: `{err}`",
        ))
    })?;
    fix_entities()?;

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// see https://github.com/SeaQL/sea-orm/pull/1947
// also we are generating an extension module from the get go
fn fix_entities() -> AppResult<()> {
    let dir = fs::read_dir("src/models/_entities")?
        .filter_map(|ent| {
            let ent = ent.unwrap();
            if ent.path().is_file()
                && ent.file_name() != "mod.rs"
                && ent.file_name() != "prelude.rs"
            {
                Some(ent.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // remove activemodel impl from all generated entities, and make note to
    // generate a new extension for those who had it
    let activemodel_exp = "impl ActiveModelBehavior for ActiveModel {}";
    let mut cleaned_entities = Vec::new();
    for file in &dir {
        let content = fs::read_to_string(file)?;
        if content.contains(activemodel_exp) {
            let content = content
                .lines()
                .filter(|line| !line.contains(activemodel_exp))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(file, content)?;
            cleaned_entities.push(file);
        }
    }

    // generate an empty extension with impl activemodel behavior
    let mut models_mod = fs::read_to_string("src/models/mod.rs")?;
    for entity_file in cleaned_entities {
        let new_file = Path::new("src/models").join(
            entity_file
                .file_name()
                .ok_or_else(|| Error::string("cannot extract file name"))?,
        );
        if !new_file.exists() {
            let module = new_file
                .file_stem()
                .ok_or_else(|| Error::string("cannot extract file stem"))?
                .to_str()
                .ok_or_else(|| Error::string("cannot extract file stem"))?;
            let module_pascal = heck::AsPascalCase(module);
            fs::write(
                &new_file,
                format!(
                    r"use sea_orm::entity::prelude::*;
pub use super::_entities::{module}::{{ActiveModel, Model, Entity}};
pub type {module_pascal} = Entity;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {{
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {{
        if !insert && self.updated_at.is_unchanged() {{
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        }} else {{
            Ok(self)
        }}
    }}
}}

// implement your read-oriented logic here
impl Model {{}}

// implement your write-oriented logic here
impl ActiveModel {{}}

// implement your custom finders, selectors oriented logic here
impl Entity {{}}
"
                ),
            )?;
            if !models_mod.contains(&format!("mod {module}")) {
                models_mod.push_str(&format!("pub mod {module};\n"));
            }
        }
    }

    fs::write("src/models/mod.rs", models_mod)?;

    Ok(())
}

/// Truncate a table in the database, effectively deleting all rows.
///
/// # Errors
///
/// Returns a [`AppResult`] if an error occurs during truncate the given table
pub async fn truncate_table<T>(db: &DatabaseConnection, _: T) -> Result<(), sea_orm::DbErr>
where
    T: EntityTrait,
{
    T::delete_many().exec(db).await?;
    Ok(())
}

/// Execute seed from the given path
///
/// # Errors
///
/// when seed process is fails
pub async fn run_app_seed<H: Hooks>(ctx: &AppContext, path: &Path) -> AppResult<()> {
    H::seed(ctx, path).await
}

/// Create a Postgres database from the given db name.
///
/// To create the database with `LOCO_POSTGRES_DB_OPTIONS`
async fn create_postgres_database(
    db_name: &str,
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    let with_options = env_vars::get_or_default(env_vars::POSTGRES_DB_OPTIONS, "ENCODING='UTF8'");

    let query = format!("CREATE DATABASE {db_name} WITH {with_options}");
    tracing::info!(query, "creating postgres database");

    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        query,
    ))
    .await?;
    Ok(())
}

/// Retrieves a list of table names from the database.
///
///
/// # Errors
///
/// Returns an error if the operation fails for any reason, such as an
/// unsupported database backend or a query execution issue.
pub async fn get_tables(db: &DatabaseConnection) -> AppResult<Vec<String>> {
    let query = match db.get_database_backend() {
        DatabaseBackend::MySql => {
            return Err(Error::Message(
                "Unsupported database backend: MySQL".to_string(),
            ))
        }
        DatabaseBackend::Postgres => {
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
        }
        DatabaseBackend::Sqlite => {
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
        }
    };

    let result = db
        .query_all(Statement::from_string(
            db.get_database_backend(),
            query.to_string(),
        ))
        .await?;

    Ok(result
        .into_iter()
        .filter_map(|row| {
            let col = match db.get_database_backend() {
                sea_orm::DatabaseBackend::MySql | sea_orm::DatabaseBackend::Postgres => {
                    "table_name"
                }
                sea_orm::DatabaseBackend::Sqlite => "name",
            };

            if let Ok(table_name) = row.try_get::<String>("", col) {
                if IGNORED_TABLES.contains(&table_name.as_str()) {
                    return None;
                }
                Some(table_name)
            } else {
                None
            }
        })
        .collect())
}

/// Dumps the contents of specified database tables into YAML files.
///
/// # Errors
/// This function retrieves data from all tables in the database, filters them
/// if `only_tables` is provided, and writes each table's content to a separate
/// YAML file in the specified directory.
///
/// Returns an error if the operation fails for any reason or could not save the
/// content into a file.
pub async fn dump_tables(
    db: &DatabaseConnection,
    to: &Path,
    only_tables: Option<Vec<String>>,
) -> AppResult<()> {
    tracing::debug!("getting tables from the database");

    let tables = get_tables(db).await?;
    tracing::info!(tables = ?tables, "found tables");

    for table in tables {
        if let Some(ref only_tables) = only_tables {
            if !only_tables.contains(&table) {
                tracing::info!(table, "skipping table as it is not in the specified list");
                continue;
            }
        }

        tracing::info!(table, "get table data");

        let data_result = db
            .query_all(Statement::from_string(
                db.get_database_backend(),
                format!(r#"SELECT * FROM "{table}""#),
            ))
            .await?;

        tracing::info!(
            table,
            rows_fetched = data_result.len(),
            "fetched rows from table"
        );

        let mut table_data: Vec<HashMap<String, serde_json::Value>> = Vec::new();

        if !to.exists() {
            tracing::info!("the specified dump folder does not exist. creating the folder now");
            fs::create_dir_all(to)?;
        }

        for row in data_result {
            let mut row_data: HashMap<String, serde_json::Value> = HashMap::new();

            for col_name in row.column_names() {
                let value_result = row
                    .try_get::<String>("", &col_name)
                    .map(serde_json::Value::String)
                    .or_else(|_| {
                        row.try_get::<i8>("", &col_name)
                            .map(serde_json::Value::from)
                    })
                    .or_else(|_| {
                        row.try_get::<i16>("", &col_name)
                            .map(serde_json::Value::from)
                    })
                    .or_else(|_| {
                        row.try_get::<i32>("", &col_name)
                            .map(serde_json::Value::from)
                    })
                    .or_else(|_| {
                        row.try_get::<i64>("", &col_name)
                            .map(serde_json::Value::from)
                    })
                    .or_else(|_| {
                        row.try_get::<f32>("", &col_name)
                            .map(serde_json::Value::from)
                    })
                    .or_else(|_| {
                        row.try_get::<f64>("", &col_name)
                            .map(serde_json::Value::from)
                    })
                    .or_else(|_| {
                        row.try_get::<uuid::Uuid>("", &col_name)
                            .map(|v| serde_json::Value::String(v.to_string()))
                    })
                    .or_else(|_| {
                        row.try_get::<DateTime<Utc>>("", &col_name)
                            .map(|v| serde_json::Value::String(v.to_rfc3339()))
                    })
                    .or_else(|_| {
                        row.try_get::<serde_json::Value>("", &col_name)
                            .map(serde_json::Value::from)
                    })
                    .or_else(|_| {
                        row.try_get::<bool>("", &col_name)
                            .map(serde_json::Value::Bool)
                    })
                    .ok();

                if let Some(value) = value_result {
                    row_data.insert(col_name, value);
                }
            }
            table_data.push(row_data);
        }

        let data = serde_yaml::to_string(&table_data)?;

        let file_db_content_path = to.join(format!("{table}.yaml"));

        let mut file = File::create(&file_db_content_path)?;
        file.write_all(data.as_bytes())?;
        tracing::info!(table, file_db_content_path = %file_db_content_path.display(), "table data written to YAML file");
    }

    tracing::info!("dumping tables process completed successfully");

    Ok(())
}

/// dumps the db schema into file.
///
/// # Errors
/// Fails with IO / sql fails
pub async fn dump_schema(ctx: &AppContext, fname: &str) -> crate::Result<()> {
    let db = &ctx.db;

    // Match the database backend and fetch schema info
    let schema_info = match db.get_database_backend() {
        DbBackend::Postgres => {
            let query = r"
                SELECT table_name, column_name, data_type
                FROM information_schema.columns
                WHERE table_schema = 'public'
                ORDER BY table_name, ordinal_position;
            ";
            let stmt = Statement::from_string(DbBackend::Postgres, query.to_owned());
            let rows = db.query_all(stmt).await?;
            rows.into_iter()
                .map(|row| {
                    // Wrap the closure in a Result to handle errors properly
                    Ok(json!({
                        "table": row.try_get::<String>("", "table_name")?,
                        "column": row.try_get::<String>("", "column_name")?,
                        "type": row.try_get::<String>("", "data_type")?,
                    }))
                })
                .collect::<Result<Vec<serde_json::Value>, DbErr>>()? // Specify error type explicitly
        }
        DbBackend::MySql => {
            let query = r"
                SELECT TABLE_NAME, COLUMN_NAME, COLUMN_TYPE
                FROM INFORMATION_SCHEMA.COLUMNS
                WHERE TABLE_SCHEMA = DATABASE()
                ORDER BY TABLE_NAME, ORDINAL_POSITION;
            ";
            let stmt = Statement::from_string(DbBackend::MySql, query.to_owned());
            let rows = db.query_all(stmt).await?;
            rows.into_iter()
                .map(|row| {
                    // Wrap the closure in a Result to handle errors properly
                    Ok(json!({
                        "table": row.try_get::<String>("", "TABLE_NAME")?,
                        "column": row.try_get::<String>("", "COLUMN_NAME")?,
                        "type": row.try_get::<String>("", "COLUMN_TYPE")?,
                    }))
                })
                .collect::<Result<Vec<serde_json::Value>, DbErr>>()? // Specify error type explicitly
        }
        DbBackend::Sqlite => {
            let query = r"
                SELECT name AS table_name, sql AS table_sql
                FROM sqlite_master
                WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                ORDER BY name;
            ";
            let stmt = Statement::from_string(DbBackend::Sqlite, query.to_owned());
            let rows = db.query_all(stmt).await?;
            rows.into_iter()
                .map(|row| {
                    // Wrap the closure in a Result to handle errors properly
                    Ok(json!({
                        "table": row.try_get::<String>("", "table_name")?,
                        "sql": row.try_get::<String>("", "table_sql")?,
                    }))
                })
                .collect::<Result<Vec<serde_json::Value>, DbErr>>()? // Specify error type explicitly
        }
    };
    // Serialize schema info to JSON format
    let schema_json = serde_json::to_string_pretty(&schema_info)?;

    // Save the schema to a file
    std::fs::write(fname, schema_json)?;
    Ok(())
}
