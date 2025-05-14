//! # Database Operations
//!
//! This module defines functions and operations related to the application's
//! database interactions.

use super::Result as AppResult;
use crate::{
    app::{AppContext, Hooks},
    cargo_config::CargoConfig,
    config, doctor, env_vars,
    errors::Error,
};
use chrono::{DateTime, Utc};
use regex::Regex;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseBackend,
    DatabaseConnection, DbBackend, DbConn, DbErr, EntityTrait, IntoActiveModel, Statement,
};
use sea_orm_migration::MigratorTrait;
use std::fmt::Write as FmtWrites;
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    fs::File,
    io::Write,
    path::Path,
    sync::OnceLock,
    time::Duration,
};
use tracing::info;

pub static EXTRACT_DB_NAME: OnceLock<Regex> = OnceLock::new();
const IGNORED_TABLES: &[&str] = &[
    "seaql_migrations",
    "pg_loco_queue",
    "sqlt_loco_queue",
    "sqlt_loco_queue_lock",
];

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

    match db.get_database_backend() {
        DatabaseBackend::Sqlite => {
            db.execute(Statement::from_string(
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
                db.execute(Statement::from_string(
                    db.get_database_backend(),
                    run_on_start.clone(),
                ))
                .await?;
            }
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

    let conn = db_uri.replace(db_name, "/postgres");
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

struct EntityCmd {
    command: Vec<String>,
    flags: BTreeMap<String, Option<String>>,
}

impl EntityCmd {
    fn new(config: &config::Database) -> Self {
        Self {
            command: vec!["generate".to_string(), "entity".to_string()],
            flags: BTreeMap::from([
                ("--database-url".to_string(), Some(config.uri.clone())),
                (
                    "--ignore-tables".to_string(),
                    Some(IGNORED_TABLES.join(",")),
                ),
                (
                    "--output-dir".to_string(),
                    Some("src/models/_entities".to_string()),
                ),
                ("--with-serde".to_string(), Some("both".to_string())),
            ]),
        }
    }

    fn merge_with_config(config: &config::Database, toml_config: &toml::Table) -> Self {
        let mut flags = Self::new(config).flags;

        for (key, value) in toml_config {
            let flag_key = format!("--{}", key.replace('_', "-"));

            // Handle special cases
            match flag_key.as_str() {
                "--output-dir" | "--database-url" => {
                    tracing::warn!(
                        "Ignoring {} configuration from Cargo.toml as it cannot be overridden",
                        key
                    );
                    continue;
                }
                "--ignore-tables" => {
                    if let (Some(current_str), Some(new_value)) = (
                        flags.get_mut(&flag_key).and_then(|c| c.as_mut()),
                        value.as_str(),
                    ) {
                        *current_str = format!("{current_str},{new_value}");
                    }
                    continue;
                }
                _ => {}
            }

            // Handle regular flags
            let flag_value = match value {
                toml::Value::String(s) => Some(s.clone()),
                toml::Value::Boolean(true) => None,
                toml::Value::Boolean(false) => continue,
                _ => Some(value.to_string()),
            };

            flags.insert(flag_key, flag_value);
        }

        Self {
            command: vec!["generate".to_string(), "entity".to_string()],
            flags,
        }
    }

    fn command(&self) -> Vec<&str> {
        let mut args: Vec<&str> = self
            .command
            .iter()
            .map(std::string::String::as_str)
            .collect();
        for (flag, value) in &self.flags {
            args.push(flag.as_str());
            if let Some(val) = value {
                args.push(val.as_str());
            }
        }
        args
    }
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

    let flags = CargoConfig::from_current_dir()?
        .get_db_entities()
        .map_or_else(
            || EntityCmd::new(&ctx.config.database),
            |entity_config| {
                tracing::info!(
                    ?entity_config,
                    "Found db.entity configuration in Cargo.toml"
                );
                EntityCmd::merge_with_config(&ctx.config.database, entity_config)
            },
        );

    let out = duct::cmd("sea-orm-cli", &flags.command())
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
                let _ = writeln!(models_mod, "pub mod {module};");
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
    let mut select = sea_orm::sea_query::Query::select();
    select
        .expr(sea_orm::sea_query::Expr::val(1))
        .from(sea_orm::sea_query::Alias::new("pg_database"))
        .and_where(
            sea_orm::sea_query::Expr::col(sea_orm::sea_query::Alias::new("datname")).eq(db_name),
        )
        .limit(1);

    let (sql, values) = select.build(sea_orm::sea_query::PostgresQueryBuilder);
    let statement = Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values);

    if db.query_one(statement).await?.is_some() {
        tracing::info!(db_name, "database already exists");

        return Err(sea_orm::DbErr::Custom("database already exists".to_owned()));
    }

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
                    .or_else(|_| row.try_get::<serde_json::Value>("", &col_name))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg::{
        config::get_database_config, db::get_value, postgres::setup_postgres_container,
    };

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

    #[tokio::test]
    async fn test_postgres_has_id_column() {
        let (pg_url, _container) = setup_postgres_container().await;
        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = pg_url;
        let db = connect(&config)
            .await
            .expect("Failed to connect to PostgreSQL");
        let backend = db.get_database_backend();

        let table_no_id = "test_table_no_id";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_no_id} (name TEXT);"),
        ))
        .await
        .expect("Failed to create table without id");

        let has_id = has_id_column(&db, &backend, table_no_id)
            .await
            .expect("Failed to check for id column");
        assert!(
            !has_id,
            "Table '{table_no_id}' should NOT have an 'id' column, but check returned true"
        );

        let table_with_id = "test_table_with_id";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_with_id} (id INTEGER PRIMARY KEY, name TEXT);"),
        ))
        .await
        .expect("Failed to create table with id");

        let has_id = has_id_column(&db, &backend, table_with_id)
            .await
            .expect("Failed to check for id column");
        assert!(
            has_id,
            "Table '{table_with_id}' SHOULD have an 'id' column, but check returned false"
        );

        let table_with_serial_id = "test_table_with_serial_id";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_with_serial_id} (id SERIAL PRIMARY KEY, name TEXT);"),
        ))
        .await
        .expect("Failed to create table with serial id");

        let has_id = has_id_column(&db, &backend, table_with_serial_id)
            .await
            .expect("Failed to check for id column");
        assert!(
            has_id,
            "Table '{table_with_serial_id}' SHOULD have an 'id' column, but check returned false"
        );
    }

    #[tokio::test]
    async fn test_sqlite_has_id_column() {
        let (config, _tree_fs) = crate::tests_cfg::config::get_sqlite_test_config("test_has_id");

        let db = connect(&config).await.expect("Failed to connect to SQLite");
        let backend = db.get_database_backend();
        assert_eq!(backend, DatabaseBackend::Sqlite);

        let table_no_id = "test_table_no_id";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_no_id} (name TEXT);"),
        ))
        .await
        .expect("Failed to create table without id");

        let has_id = has_id_column(&db, &backend, table_no_id)
            .await
            .expect("Failed to check for id column");
        assert!(
            !has_id,
            "Table '{table_no_id}' should NOT have an 'id' column, but check returned true"
        );

        let table_with_id = "test_table_with_id";
        db.execute(Statement::from_string(
            backend,
            // SQLite uses INTEGER PRIMARY KEY for rowid alias
            format!("CREATE TABLE {table_with_id} (id INTEGER PRIMARY KEY, name TEXT);"),
        ))
        .await
        .expect("Failed to create table with id");

        let has_id = has_id_column(&db, &backend, table_with_id)
            .await
            .expect("Failed to check for id column");
        assert!(
            has_id,
            "Table '{table_with_id}' SHOULD have an 'id' column, but check returned false"
        );

        let table_with_auto_id = "test_table_with_auto_id";
        db.execute(Statement::from_string(
            backend,
            // AUTOINCREMENT keyword is important for SQLite's sequence behavior
            format!("CREATE TABLE {table_with_auto_id} (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT);"),
        ))
        .await
        .expect("Failed to create table with auto id");

        let has_id = has_id_column(&db, &backend, table_with_auto_id)
            .await
            .expect("Failed to check for id column");
        assert!(
            has_id,
            "Table '{table_with_auto_id}' SHOULD have an 'id' column, but check returned false"
        );
    }

    #[tokio::test]
    async fn test_postgres_is_auto_increment() {
        let (pg_url, _container) = setup_postgres_container().await;
        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = pg_url;
        let db = connect(&config)
            .await
            .expect("Failed to connect to PostgreSQL");
        let backend = db.get_database_backend();

        let table_no_id = "test_table_no_id_auto";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_no_id} (name TEXT);"),
        ))
        .await
        .expect("Failed to create table without id");

        let has_id = has_id_column(&db, &backend, table_no_id)
            .await
            .expect("Failed to check for id column existence");
        assert!(
            !has_id,
            "Table '{table_no_id}' should not have an 'id' column."
        );

        let auto_inc_result = is_auto_increment(&db, &backend, table_no_id).await;
        assert!(
            auto_inc_result.is_err(),
            "is_auto_increment should error if 'id' column doesn't exist, but it returned Ok"
        );

        let table_with_id_not_auto = "test_table_id_not_auto";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_with_id_not_auto} (id INTEGER PRIMARY KEY, name TEXT);"),
        ))
        .await
        .expect("Failed to create table with non-auto id");

        let is_auto = is_auto_increment(&db, &backend, table_with_id_not_auto)
            .await
            .expect("Failed to check auto-increment");
        assert!(
            !is_auto,
            "Table '{table_with_id_not_auto}' should NOT be auto-increment, but check returned true"
        );

        let table_with_serial_id = "test_table_serial_id_auto";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_with_serial_id} (id SERIAL PRIMARY KEY, name TEXT);"),
        ))
        .await
        .expect("Failed to create table with serial id");

        let is_auto = is_auto_increment(&db, &backend, table_with_serial_id)
            .await
            .expect("Failed to check auto-increment");
        assert!(
            is_auto,
            "Table '{table_with_serial_id}' SHOULD be auto-increment, but check returned false"
        );
    }

    #[tokio::test]
    async fn test_postgres_reset_autoincrement() {
        // Setup PostgreSQL container
        let (pg_url, _container) = setup_postgres_container().await;
        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = pg_url;
        let db = connect(&config)
            .await
            .expect("Failed to connect to PostgreSQL");
        let backend = db.get_database_backend();

        // Create test table with SERIAL id
        let table_name = "test_reset_sequence";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_name} (id SERIAL PRIMARY KEY, name TEXT);"),
        ))
        .await
        .expect("Failed to create test table");

        // Insert multiple rows in a single query
        db.execute(Statement::from_string(
            backend,
            format!("INSERT INTO {table_name} (name) VALUES ('one'), ('two'), ('three');"),
        ))
        .await
        .expect("Failed to insert test data");

        // Delete all rows
        db.execute(Statement::from_string(
            backend,
            format!("DELETE FROM {table_name};"),
        ))
        .await
        .expect("Failed to delete rows");

        // Insert a new row and check ID (should be 4, continuing the sequence)
        let result = db
            .query_one(Statement::from_string(
                backend,
                format!("INSERT INTO {table_name} (name) VALUES ('test') RETURNING id;"),
            ))
            .await
            .expect("Failed to insert row")
            .expect("No row returned");

        let id = result.try_get::<i32>("", "id").expect("Failed to get ID");
        assert_eq!(
            id, 4,
            "ID should be 4 after insert (sequence was not reset)"
        );

        // Delete all rows again
        db.execute(Statement::from_string(
            backend,
            format!("DELETE FROM {table_name};"),
        ))
        .await
        .expect("Failed to delete rows");

        // Reset auto-increment sequence
        reset_autoincrement(backend, table_name, &db)
            .await
            .expect("Failed to reset sequence");

        // Insert a new row and check ID (should be 1 after reset)
        let result = db
            .query_one(Statement::from_string(
                backend,
                format!("INSERT INTO {table_name} (name) VALUES ('reset') RETURNING id;"),
            ))
            .await
            .expect("Failed to insert row")
            .expect("No row returned");

        let id = result.try_get::<i32>("", "id").expect("Failed to get ID");
        assert_eq!(id, 1, "ID should be 1 after sequence reset");
    }

    #[tokio::test]
    async fn test_sqlite_reset_autoincrement() {
        // Setup SQLite database
        let (config, _tree_fs) = crate::tests_cfg::config::get_sqlite_test_config("test_reset");

        let db = connect(&config).await.expect("Failed to connect to SQLite");
        let backend = db.get_database_backend();

        // Create test table with auto-incrementing id
        let table_name = "test_reset_sequence";
        db.execute(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_name} (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT);"),
        ))
        .await
        .expect("Failed to create test table");

        // Insert multiple rows in a single query
        db.execute(Statement::from_string(
            backend,
            format!("INSERT INTO {table_name} (name) VALUES ('one'), ('two'), ('three');"),
        ))
        .await
        .expect("Failed to insert test data");

        // Delete all rows
        db.execute(Statement::from_string(
            backend,
            format!("DELETE FROM {table_name};"),
        ))
        .await
        .expect("Failed to delete rows");

        // Insert a new row and check ID (should be 4, continuing the sequence)
        let result = db
            .query_one(Statement::from_string(
                backend,
                format!("INSERT INTO {table_name} (name) VALUES ('test') RETURNING id;"),
            ))
            .await
            .expect("Failed to insert row")
            .expect("No row returned");

        let id = result.try_get::<i32>("", "id").expect("Failed to get ID");
        assert_eq!(
            id, 4,
            "ID should be 4 after insert (sequence was not reset)"
        );

        // Delete all rows again
        db.execute(Statement::from_string(
            backend,
            format!("DELETE FROM {table_name};"),
        ))
        .await
        .expect("Failed to delete rows");

        // Reset auto-increment sequence
        reset_autoincrement(backend, table_name, &db)
            .await
            .expect("Failed to reset sequence");

        // Insert a new row and check ID (should be 1 after reset)
        let result = db
            .query_one(Statement::from_string(
                backend,
                format!("INSERT INTO {table_name} (name) VALUES ('reset') RETURNING id;"),
            ))
            .await
            .expect("Failed to insert row")
            .expect("No row returned");

        let id = result.try_get::<i32>("", "id").expect("Failed to get ID");
        assert_eq!(id, 1, "ID should be 1 after sequence reset");
    }

    #[test]
    fn test_entity_cmd_new() {
        let cmd = EntityCmd::new(&get_database_config());

        let expected = "generate entity --database-url sqlite::memory: --ignore-tables \
            seaql_migrations,pg_loco_queue,sqlt_loco_queue,sqlt_loco_queue_lock --output-dir \
            src/models/_entities --with-serde both";
        assert_eq!(cmd.command().join(" "), expected);
    }

    #[test]
    fn test_entity_cmd_merge_with_config() {
        let config_str = r#"
max-connections = "1"
ignore-tables = "table1,table2"
with-serde = "none"
model-extra-derives = "ts_rs::Ts"
"#;
        let config: toml::Table = toml::from_str(config_str).unwrap();

        let cmd = EntityCmd::merge_with_config(&get_database_config(), &config);

        let expected = "generate entity --database-url sqlite::memory: --ignore-tables \
            seaql_migrations,pg_loco_queue,sqlt_loco_queue,sqlt_loco_queue_lock,table1,table2 \
            --max-connections 1 --model-extra-derives ts_rs::Ts --output-dir src/models/_entities \
            --with-serde none";
        assert_eq!(cmd.command().join(" "), expected);
    }
}
