use std::{fs::File, path::Path};

use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, EntityName,
    EntityTrait, IntoActiveModel, Statement,
};
use serde_json::Value;

use crate::{
    app::{AppContext, Hooks},
    Result as AppResult,
};

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
    for<'de> <<A as ActiveModelTrait>::Entity as EntityTrait>::Model:
        serde::de::Deserialize<'de> + serde::Serialize,
    A: ActiveModelTrait + Send + Sync,
    A: sea_orm::TryIntoModel<<<A as ActiveModelTrait>::Entity as EntityTrait>::Model>,
    sea_orm::Insert<A>: Send + Sync,
    <A as ActiveModelTrait>::Entity: EntityName,
{
    // Deserialize YAML file into a vector of JSON values
    let seed_data: Vec<Value> = serde_yaml::from_reader(File::open(path)?)?;

    // Insert each row
    let mut seed_models = Vec::new();
    for row in seed_data {
        let model = A::from_json(row)?;
        seed_models.push(model);
    }
    A::Entity::insert_many(seed_models).exec(db).await?;

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
/// common primary key column. It supports the `Postgres` and `SQLite`
/// backends; any other backend returns [`sea_orm::DbErr::BackendNotSupported`].
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
                .query_one_raw(Statement::from_string(DatabaseBackend::Postgres, query))
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
                .query_one_raw(Statement::from_string(DatabaseBackend::Sqlite, query))
                .await?;
            result.is_some_and(|row| row.try_get::<i32>("", "count").unwrap_or(0) > 0)
        }
        bk => {
            return Err(DbErr::BackendNotSupported {
                db: bk.as_str(),
                ctx: "has_id_column",
            }
            .into());
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
                .query_one_raw(Statement::from_string(DatabaseBackend::Postgres, query))
                .await?;
            result.is_some_and(|row| row.try_get::<bool>("", "is_serial").unwrap_or(false))
        }
        DatabaseBackend::Sqlite => {
            let query =
                format!("SELECT sql FROM sqlite_master WHERE type='table' AND name='{table_name}'");
            let result = db
                .query_one_raw(Statement::from_string(DatabaseBackend::Sqlite, query))
                .await?;
            result.is_some_and(|row| {
                row.try_get::<String>("", "sql")
                    .is_ok_and(|sql| sql.to_lowercase().contains("autoincrement"))
            })
        }
        bk => {
            return Err(DbErr::BackendNotSupported {
                db: bk.as_str(),
                ctx: "is_auto_increment",
            }
            .into());
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
            db.execute_raw(Statement::from_sql_and_values(
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
            db.execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                &query_str,
                vec![],
            ))
            .await?;
        }
        bk => {
            return Err(DbErr::BackendNotSupported {
                db: bk.as_str(),
                ctx: "reset_autoincrement",
            }
            .into());
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg::postgres::setup_postgres_container;

    #[tokio::test]
    async fn test_postgres_has_id_column() {
        let (pg_url, _container) = setup_postgres_container().await;
        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = pg_url;
        let db = crate::db::connect(&config)
            .await
            .expect("Failed to connect to PostgreSQL");
        let backend = db.get_database_backend();

        let table_no_id = "test_table_no_id";
        db.execute_raw(Statement::from_string(
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
        db.execute_raw(Statement::from_string(
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
        db.execute_raw(Statement::from_string(
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

        let db = crate::db::connect(&config)
            .await
            .expect("Failed to connect to SQLite");
        let backend = db.get_database_backend();
        assert_eq!(backend, DatabaseBackend::Sqlite);

        let table_no_id = "test_table_no_id";
        db.execute_raw(Statement::from_string(
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
        db.execute_raw(Statement::from_string(
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
        db.execute_raw(Statement::from_string(
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
        let db = crate::db::connect(&config)
            .await
            .expect("Failed to connect to PostgreSQL");
        let backend = db.get_database_backend();

        let table_no_id = "test_table_no_id_auto";
        db.execute_raw(Statement::from_string(
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
        db.execute_raw(Statement::from_string(
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
        db.execute_raw(Statement::from_string(
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
        let db = crate::db::connect(&config)
            .await
            .expect("Failed to connect to PostgreSQL");
        let backend = db.get_database_backend();

        // Create test table with SERIAL id
        let table_name = "test_reset_sequence";
        db.execute_raw(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_name} (id SERIAL PRIMARY KEY, name TEXT);"),
        ))
        .await
        .expect("Failed to create test table");

        // Insert multiple rows in a single query
        db.execute_raw(Statement::from_string(
            backend,
            format!("INSERT INTO {table_name} (name) VALUES ('one'), ('two'), ('three');"),
        ))
        .await
        .expect("Failed to insert test data");

        // Delete all rows
        db.execute_raw(Statement::from_string(
            backend,
            format!("DELETE FROM {table_name};"),
        ))
        .await
        .expect("Failed to delete rows");

        // Insert a new row and check ID (should be 4, continuing the sequence)
        let result = db
            .query_one_raw(Statement::from_string(
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
        db.execute_raw(Statement::from_string(
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
            .query_one_raw(Statement::from_string(
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

        let db = crate::db::connect(&config)
            .await
            .expect("Failed to connect to SQLite");
        let backend = db.get_database_backend();

        // Create test table with auto-incrementing id
        let table_name = "test_reset_sequence";
        db.execute_raw(Statement::from_string(
            backend,
            format!("CREATE TABLE {table_name} (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT);"),
        ))
        .await
        .expect("Failed to create test table");

        // Insert multiple rows in a single query
        db.execute_raw(Statement::from_string(
            backend,
            format!("INSERT INTO {table_name} (name) VALUES ('one'), ('two'), ('three');"),
        ))
        .await
        .expect("Failed to insert test data");

        // Delete all rows
        db.execute_raw(Statement::from_string(
            backend,
            format!("DELETE FROM {table_name};"),
        ))
        .await
        .expect("Failed to delete rows");

        // Insert a new row and check ID (should be 4, continuing the sequence)
        let result = db
            .query_one_raw(Statement::from_string(
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
        db.execute_raw(Statement::from_string(
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
            .query_one_raw(Statement::from_string(
                backend,
                format!("INSERT INTO {table_name} (name) VALUES ('reset') RETURNING id;"),
            ))
            .await
            .expect("Failed to insert row")
            .expect("No row returned");

        let id = result.try_get::<i32>("", "id").expect("Failed to get ID");
        assert_eq!(id, 1, "ID should be 1 after sequence reset");
    }
}
