//! # Database Operations
//!
//! This module defines functions and operations related to the application's database interactions.
//!
use crate::{app::Hooks, config};

use super::Result as AppResult;
use duct::cmd;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, DbConn, EntityTrait,
    IntoActiveModel,
};
use sea_orm_migration::MigratorTrait;
use std::fs::File;
use tracing::info;

/// converge database logic
///
/// # Errors
///
///  an `AppResult`, which is an alias for `Result<(), AppError>`. It may
/// return an `AppError` variant representing different database operation failures.
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

/// Establish a connection to the database using the provided configuration settings.
///
/// # Errors
///
/// Returns a [`sea_orm::DbErr`] if an error occurs during the database connection establishment.
pub async fn connect(config: &config::Database) -> Result<DbConn, sea_orm::DbErr> {
    let mut opt = ConnectOptions::new(&config.uri);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .sqlx_logging(config.enable_logging);

    Database::connect(opt).await
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

/// Reset the database, dropping and recreating the schema and applying migrations.
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
/// Returns a [`AppResult`] if could not render the path content into [`Vec<serde_json::Value>`] or could not inset the vector to DB.
pub async fn seed<A>(db: &DatabaseConnection, path: &str) -> AppResult<()>
where
    <<A as ActiveModelTrait>::Entity as EntityTrait>::Model: IntoActiveModel<A>,
    for<'de> <<A as ActiveModelTrait>::Entity as EntityTrait>::Model: serde::de::Deserialize<'de>,
    A: sea_orm::ActiveModelTrait,
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
pub fn entities<M: MigratorTrait>(_db: &DatabaseConnection) -> AppResult<String> {
    let out = cmd!(
        "sea-orm-cli",
        "generate",
        "entity",
        "--output-dir",
        "src/models/_entities",
    )
    .stderr_to_stdout()
    .run()?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
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
