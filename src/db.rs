//! # Database Operations
//!
//! This module defines functions and operations related to the application's
//! database interactions.

use std::{fs::File, path::Path, time::Duration};

use duct::cmd;
use fs_err as fs;
use lazy_static::lazy_static;
use regex::Regex;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseBackend,
    DatabaseConnection, DbConn, EntityTrait, IntoActiveModel, Statement,
};
use sea_orm_migration::MigratorTrait;
use tracing::info;

use super::Result as AppResult;
use crate::{
    app::{AppContext, Hooks},
    config, doctor,
    errors::Error,
};

lazy_static! {
    // Getting the table name from the environment configuration.
    // For example:
    // postgres://loco:loco@localhost:5432/loco_app
    // mysql://loco:loco@localhost:3306/loco_app
    // the results will be loco_app
    pub static ref EXTRACT_DB_NAME: Regex = Regex::new(r"/([^/]+)$").unwrap();
}

/// Verifies a user has access to data within its database
///
/// # Errors
///
/// This function will return an error if IO fails
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
        DatabaseConnection::SqlxMySqlPoolConnection(_) => {}
        DatabaseConnection::SqlxSqlitePoolConnection(_) => {}
        DatabaseConnection::Disconnected => {
            return Err(Error::string("connection to database has been closed"));
        }
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
    db: &DatabaseConnection,
    config: &config::Database,
) -> AppResult<()> {
    if config.dangerously_recreate {
        info!("recreating schema");
        reset::<M>(db).await?;
        return Ok(());
    }

    if config.auto_migrate {
        info!("auto migrating");
        migrate::<M>(db).await?;
    }

    if config.dangerously_truncate {
        info!("truncating tables");
        H::truncate(db).await?;
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

    Database::connect(opt).await
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
    let db_name = EXTRACT_DB_NAME
        .captures(db_uri)
        .and_then(|cap| cap.get(1).map(|db| db.as_str()))
        .ok_or_else(|| {
            Error::string(
                "The specified table name was not found in the given Postgre database URI",
            )
        })?;

    let conn = EXTRACT_DB_NAME.replace(db_uri, "/postgres").to_string();
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
pub async fn seed<A>(db: &DatabaseConnection, path: &str) -> AppResult<()>
where
    <<A as ActiveModelTrait>::Entity as EntityTrait>::Model: IntoActiveModel<A>,
    for<'de> <<A as ActiveModelTrait>::Entity as EntityTrait>::Model: serde::de::Deserialize<'de>,
    A: sea_orm::ActiveModelTrait + Send + Sync,
    sea_orm::Insert<A>: Send + Sync, // Add this Send bound
{
    let loader: Vec<serde_json::Value> = serde_yaml::from_reader(File::open(path)?)?;

    let mut users: Vec<A> = vec![];
    for user in loader {
        users.push(A::from_json(user)?);
    }

    <A as ActiveModelTrait>::Entity::insert_many(users)
        .exec(db)
        .await?;

    Ok(())
}

/// Generate entity model.
/// This function using sea-orm-cli.
///
/// # Errors
///
/// Returns a [`AppResult`] if an error occurs during generate model entity.
pub async fn entities<M: MigratorTrait>(ctx: &AppContext) -> AppResult<String> {
    doctor::check_seaorm_cli().to_result()?;
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
        &ctx.config.database.uri
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
        .flatten()
        .filter(|ent| {
            ent.path().is_file() && ent.file_name() != "mod.rs" && ent.file_name() != "prelude.rs"
        })
        .map(|ent| ent.path())
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
            fs::write(
                &new_file,
                format!(
                    r"use sea_orm::entity::prelude::*;
use super::_entities::{module}::ActiveModel;

impl ActiveModelBehavior for ActiveModel {{
    // extend activemodel below (keep comment for generators)
}}
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
pub async fn run_app_seed<H: Hooks>(db: &DatabaseConnection, path: &Path) -> AppResult<()> {
    H::seed(db, path).await
}

/// Create a Postgres table from the given table name.
///
/// To create the table with `LOCO_POSTGRES_TABLE_OPTIONS`
async fn create_postgres_database(
    table_name: &str,
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    let with_options = std::env::var("LOCO_POSTGRES_TABLE_OPTIONS")
        .map_or_else(|_| "ENCODING='UTF8'".to_string(), |options| options);

    let query = format!("CREATE DATABASE {table_name} WITH {with_options}");
    tracing::info!(query, "creating postgres table");

    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        query,
    ))
    .await?;
    Ok(())
}
