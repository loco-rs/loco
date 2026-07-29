use sea_orm::DatabaseConnection;
use sea_orm_migration::MigratorTrait;
use tracing::info;

use crate::{
    app::{AppContext, Hooks},
    config, Result as AppResult,
};

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
