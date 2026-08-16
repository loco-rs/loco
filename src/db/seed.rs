use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use futures_util::TryStreamExt;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, EntityName,
    EntityTrait, IntoActiveModel, Statement,
};
use serde_json::Value;

use crate::{
    app::{AppContext, Hooks},
    Result as AppResult,
};

/// Truncate a table in the database, effectively deleting all rows.
///
/// # Errors
///
/// Returns a [`AppResult`] if an error occurs during truncate the given table
pub async fn truncate_table<T>(db: &DatabaseConnection, _: T) -> Result<(), DbErr>
where
    T: EntityTrait,
{
    T::delete_many().exec(db).await?;
    Ok(())
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
    // Deserialize YAML file into a vector of JSON values. Read the file
    // asynchronously (whole-file into memory — seed fixtures are small) rather
    // than blocking the async runtime on a synchronous `File`/`from_reader`.
    let seed_bytes = tokio::fs::read(path).await?;
    let seed_data: Vec<Value> = serde_yaml::from_slice(&seed_bytes)?;

    // Insert each row.
    //
    // NOTE: we deserialize the fixture row straight into the entity `Model` and
    // then convert, instead of `ActiveModelTrait::from_json`. On sea-orm
    // 2.0, `from_json` first merges every column's *default* value via
    // `sea_query::sea_value_to_json_value`, which renders chrono datetimes as
    // SQL-literal strings (`'1970-01-01 00:00:00.000000 +00:00'`) and NULLs as
    // the literal string `"NULL"`. Those then fail chrono's RFC3339 serde parse
    // ("input contains invalid characters") when the merged object is
    // deserialized back into the `Model`, which breaks seeding for ANY entity
    // with datetime columns. Deserializing the row directly skips the broken
    // default-merge (fixtures already carry every required column). Revisit once
    // the upstream sea-orm fix lands.
    let mut seed_models = Vec::new();
    for row in seed_data {
        let model: <<A as ActiveModelTrait>::Entity as EntityTrait>::Model =
            serde_json::from_value(row)?;
        seed_models.push(model.into_active_model());
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

/// Dump an entity's rows to a YAML fixture at `path`, symmetric to [`seed`].
///
/// Rows are streamed from the database and serialized one at a time straight to
/// a buffered writer, so memory stays bounded to a single row regardless of
/// table size (unlike [`crate::db::dump_tables`], which loads and buffers the
/// whole table). Because each row is serialized through its typed entity
/// `Model`, datetimes, UUIDs, JSON and booleans are emitted exactly as [`seed`]
/// expects to read them back — the dump/seed round-trip is symmetric by
/// construction (see #1691).
///
/// Wire it into [`crate::app::Hooks::dump`] to control which entities are
/// dumped and in what order:
///
/// ```ignore
/// async fn dump(ctx: &AppContext, base: &Path) -> Result<()> {
///     db::dump::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").to_string_lossy()).await?;
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns an error if the query stream fails or the file cannot be written.
pub async fn dump<A>(db: &DatabaseConnection, path: &str) -> crate::Result<()>
where
    <<A as ActiveModelTrait>::Entity as EntityTrait>::Model: serde::Serialize,
    A: ActiveModelTrait + Send + Sync,
    <A as ActiveModelTrait>::Entity: EntityName,
{
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let mut stream = A::Entity::find().stream(db).await?;
    let mut wrote_any = false;
    while let Some(model) = stream.try_next().await? {
        // Serialize each row on its own, then re-emit it as a `- `-prefixed YAML
        // sequence item. Per-row `to_string` keeps memory bounded and avoids
        // serde_yaml's streaming `Serializer`, which cannot be held across the
        // stream's awaits.
        let yaml = serde_yaml::to_string(&model)?;
        let mut first = true;
        for line in yaml.lines() {
            // serde_yaml emits a plain mapping (no `---`), but guard anyway.
            if line == "---" {
                continue;
            }
            if first {
                writeln!(writer, "- {line}")?;
                first = false;
            } else {
                // Indent continuation lines two spaces to nest under the item.
                writeln!(writer, "  {line}")?;
            }
        }
        wrote_any = true;
    }

    // An empty table is a valid, re-seedable empty sequence.
    if !wrote_any {
        writer.write_all(b"[]\n")?;
    }

    writer.flush()?;
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

/// Execute a dump from the given base directory via [`Hooks::dump`].
///
/// # Errors
///
/// When the dump process fails.
pub async fn run_app_dump<H: Hooks>(ctx: &AppContext, base: &Path) -> AppResult<()> {
    H::dump(ctx, base).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg::postgres::setup_postgres_container;

    // Entity with datetime + JSON columns to prove the typed `dump` round-trips
    // through `seed` with full fidelity.
    mod typed_dump_entity {
        use sea_orm::entity::prelude::*;

        #[derive(
            Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize,
        )]
        #[sea_orm(table_name = "typed_dump")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub name: String,
            pub created_at: DateTimeWithTimeZone,
            pub payload: Json,
        }

        #[derive(Copy, Clone, Debug, EnumIter)]
        pub enum Relation {}

        impl RelationTrait for Relation {
            fn def(&self) -> RelationDef {
                panic!("no relations for typed_dump_entity::Relation")
            }
        }

        impl ActiveModelBehavior for ActiveModel {}
    }

    #[tokio::test]
    async fn sqlite_typed_dump_roundtrips() {
        use sea_orm::{ActiveValue::Set, QueryOrder};

        let (config, tree_fs) = crate::tests_cfg::config::get_sqlite_test_config("typed_dump");
        let db = crate::db::connect(&config).await.expect("connect sqlite");
        let backend = db.get_database_backend();

        db.execute_raw(Statement::from_string(
            backend,
            "CREATE TABLE typed_dump (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, created_at TEXT NOT NULL, payload TEXT NOT NULL);".to_string(),
        ))
        .await
        .expect("create typed_dump table");

        let ts =
            chrono::DateTime::parse_from_rfc3339("2026-02-01T08:00:00+00:00").expect("parse ts");
        for (name, n) in [("alice", 1), ("bob", 2)] {
            typed_dump_entity::ActiveModel {
                name: Set(name.to_string()),
                created_at: Set(ts),
                payload: Set(serde_json::json!({ "n": n })),
                ..Default::default()
            }
            .insert(&db)
            .await
            .expect("insert row");
        }

        let dump_dir = tree_fs.root.join("dump");
        std::fs::create_dir_all(&dump_dir).expect("create dump dir");
        let path = dump_dir.join("typed_dump.yaml");
        let path_str = path.to_str().expect("utf8 path");

        crate::db::dump::<typed_dump_entity::ActiveModel>(&db, path_str)
            .await
            .expect("typed dump failed");

        // The typed dump must parse as a YAML sequence and carry an RFC3339
        // datetime (serialized through the typed Model, not raw SQLite text).
        let contents = std::fs::read_to_string(&path).expect("read dump");
        let parsed: Vec<serde_json::Value> =
            serde_yaml::from_str(&contents).expect("dump must be a valid YAML sequence");
        assert_eq!(parsed.len(), 2, "two rows dumped");
        // Serialized through the typed Model, so it is valid RFC3339 (chrono
        // renders UTC as `Z`) rather than raw SQLite text.
        let dumped_dt = parsed[0]["created_at"].as_str().expect("created_at string");
        assert_eq!(
            chrono::DateTime::parse_from_rfc3339(dumped_dt)
                .expect("dumped datetime is RFC3339")
                .to_utc(),
            ts.to_utc(),
            "dumped datetime must equal the original instant, got {dumped_dt}"
        );

        // Round-trip: truncate, re-seed from the dump, and compare.
        db.execute_raw(Statement::from_string(
            backend,
            "DELETE FROM typed_dump;".to_string(),
        ))
        .await
        .expect("truncate");
        crate::db::seed::<typed_dump_entity::ActiveModel>(&db, path_str)
            .await
            .expect("re-seed from typed dump");

        let rows = typed_dump_entity::Entity::find()
            .order_by_asc(typed_dump_entity::Column::Id)
            .all(&db)
            .await
            .expect("select after seed");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "alice");
        assert_eq!(rows[0].created_at, ts);
        assert_eq!(rows[0].payload, serde_json::json!({ "n": 1 }));
        assert_eq!(rows[1].name, "bob");
        assert_eq!(rows[1].payload, serde_json::json!({ "n": 2 }));
    }

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
