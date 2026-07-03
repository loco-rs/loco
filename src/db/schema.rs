use std::{
    collections::{BTreeMap, HashSet},
    fs,
    fs::File,
    io::Write,
    path::Path,
};

use chrono::{DateTime, Utc};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DbBackend, DbErr, EntityTrait, Statement,
};
use serde_json::json;

use super::IGNORED_TABLES;
use crate::{app::AppContext, Result as AppResult};

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

/// Retrieves a list of table names from the database.
///
///
/// # Errors
///
/// Returns an error if the operation fails for any reason, such as an
/// unsupported database backend or a query execution issue.
pub async fn get_tables(db: &DatabaseConnection) -> AppResult<Vec<String>> {
    let query = match db.get_database_backend() {
        DatabaseBackend::Postgres => {
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
        }
        DatabaseBackend::Sqlite => {
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
        }
        bk => {
            return Err(DbErr::BackendNotSupported {
                db: bk.as_str(),
                ctx: "get_tables",
            }
            .into());
        }
    };

    let result = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            query.to_string(),
        ))
        .await?;

    Ok(result
        .into_iter()
        .filter_map(|row| {
            let col = match db.get_database_backend() {
                sea_orm::DatabaseBackend::Postgres => "table_name",
                sea_orm::DatabaseBackend::Sqlite => "name",
                _ => unreachable!(),
            };

            match row.try_get::<String>("", col) {
                Ok(table_name) => {
                    if IGNORED_TABLES.contains(&table_name.as_str()) {
                        return None;
                    }
                    Some(table_name)
                }
                _ => None,
            }
        })
        .collect())
}

/// Returns the set of column names that are of boolean type for the given table.
///
/// This uses lightweight schema metadata queries and is used by the dump logic
/// to serialize boolean columns as `true` / `false` instead of `0` / `1`.
async fn get_boolean_columns(
    db: &DatabaseConnection,
    table_name: &str,
) -> AppResult<HashSet<String>> {
    let backend = db.get_database_backend();
    let mut bool_cols = HashSet::new();

    match backend {
        DatabaseBackend::Postgres => {
            let query = "
                SELECT column_name
                FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = $1
                  AND data_type = 'boolean'
            ";

            let stmt = Statement::from_sql_and_values(backend, query, vec![table_name.into()]);
            let rows = db.query_all_raw(stmt).await?;
            for row in rows {
                if let Ok(col) = row.try_get::<String>("", "column_name") {
                    bool_cols.insert(col);
                }
            }
        }
        DatabaseBackend::Sqlite => {
            let query = format!("PRAGMA table_info('{table_name}')");
            let stmt = Statement::from_string(backend, query);
            let rows = db.query_all_raw(stmt).await?;
            for row in rows {
                let col_name = row.try_get::<String>("", "name")?;
                let col_type: String = row.try_get::<String>("", "type").unwrap_or_default();
                if col_type.to_ascii_uppercase().contains("BOOL") {
                    bool_cols.insert(col_name);
                }
            }
        }
        bk => {
            return Err(DbErr::BackendNotSupported {
                db: bk.as_str(),
                ctx: "get_boolean_columns",
            }
            .into());
        }
    }

    Ok(bool_cols)
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
#[allow(clippy::cognitive_complexity)]
pub async fn dump_tables(
    db: &DatabaseConnection,
    to: &Path,
    only_tables: Option<Vec<String>>,
) -> AppResult<()> {
    tracing::debug!("getting tables from the database");

    let tables = get_tables(db).await?;
    tracing::info!(tables = ?tables, "found tables");

    for table in tables {
        if let Some(ref only_tables) = only_tables
            && !only_tables.contains(&table)
        {
            tracing::info!(table, "skipping table as it is not in the specified list");
            continue;
        }

        tracing::info!(table, "get table data");

        // Discover which columns are booleans so we can dump them as true/false
        // instead of 0/1 while keeping numeric IDs and other integers intact.
        let boolean_columns = get_boolean_columns(db, &table).await?;

        let data_result = db
            .query_all_raw(Statement::from_string(
                db.get_database_backend(),
                format!(r#"SELECT * FROM "{table}""#),
            ))
            .await?;

        tracing::info!(
            table,
            rows_fetched = data_result.len(),
            "fetched rows from table"
        );

        let mut table_data: Vec<BTreeMap<String, serde_json::Value>> = Vec::new();

        if !to.exists() {
            tracing::info!("the specified dump folder does not exist. creating the folder now");
            fs::create_dir_all(to)?;
        }

        for row in data_result {
            // Use BTreeMap to ensure deterministic key order in YAML snapshots.
            let mut row_data: BTreeMap<String, serde_json::Value> = BTreeMap::new();

            for col_name in row.column_names() {
                let value_result = row
                    // Try native JSON value first so JSON/JSONB columns and JSON-like
                    // text (e.g. '{\"k\": \"v\"}', '[1,2,3]') are captured as structured
                    // data instead of being double-quoted strings in YAML.
                    .try_get::<serde_json::Value>("", &col_name)
                    // Fallback to string and scalar types for non-JSON columns.
                    .or_else(|_| {
                        row.try_get::<String>("", &col_name)
                            .map(serde_json::Value::String)
                    })
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
                    .ok();

                if let Some(mut value) = value_result {
                    if boolean_columns.contains(&col_name)
                        && let serde_json::Value::Number(num) = &value
                        && let Some(i) = num.as_i64()
                    {
                        value = serde_json::Value::Bool(i != 0);
                    }

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
            let rows = db.query_all_raw(stmt).await?;
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
            let rows = db.query_all_raw(stmt).await?;
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
            let rows = db.query_all_raw(stmt).await?;
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
        db => {
            return Err(DbErr::BackendNotSupported {
                db: db.as_str(),
                ctx: "dump_schema",
            }
            .into());
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

    // Minimal SeaORM entity for the dump_types test table to exercise the
    // full dump -> seed -> select roundtrip using the same seed() logic as
    // the CLI.
    mod dump_types_entity {
        use sea_orm::entity::prelude::*;

        #[derive(
            Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize,
        )]
        #[sea_orm(table_name = "dump_types")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub active: bool,
            pub counter: i32,
            pub rating: f64,
            pub name: String,
            pub created_at: String,
            pub uuid_col: String,
            pub json_col: Json,
            pub opt_text: Option<String>,
            pub opt_int: Option<i32>,
            pub opt_bool: Option<bool>,
            pub array_col: Json,
        }

        #[derive(Copy, Clone, Debug, EnumIter)]
        pub enum Relation {}

        impl RelationTrait for Relation {
            fn def(&self) -> RelationDef {
                panic!("no relations for dump_types_entity::Relation")
            }
        }

        impl ActiveModelBehavior for ActiveModel {}
    }

    #[tokio::test]
    async fn sqlite_dump_tables_roundtrip() {
        use dump_types_entity::ActiveModel as DumpTypesActiveModel;
        use dump_types_entity::Entity as DumpTypesEntity;
        use insta::assert_snapshot;
        use sea_orm::QueryOrder;

        use crate::tests_cfg::config::get_sqlite_test_config;

        // Arrange: create a temporary SQLite database with a table that has
        // a variety of column types we support in loco.
        let (config, tree_fs) = get_sqlite_test_config("dump_types");
        let db = crate::db::connect(&config)
            .await
            .expect("Failed to connect to SQLite test database");
        let backend = db.get_database_backend();
        assert_eq!(backend, DatabaseBackend::Sqlite);

        let table_name = "dump_types";

        // Create table with representative types:
        // - integer PK
        // - boolean (required + optional)
        // - integer (required + optional)
        // - real (required)
        // - text (required + optional)
        // - created_at (text datetime-like)
        // - uuid-like text
        // - json-like text (object)
        // - array-like text (JSON array)
        db.execute_raw(Statement::from_string(
            backend,
            format!(
                "CREATE TABLE {table_name} (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    active BOOLEAN NOT NULL,
                    counter INTEGER NOT NULL,
                    rating REAL NOT NULL,
                    name TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    uuid_col TEXT NOT NULL,
                    json_col TEXT NOT NULL,
                    opt_text TEXT,
                    opt_int INTEGER,
                    opt_bool BOOLEAN,
                    array_col TEXT NOT NULL
                );"
            ),
        ))
        .await
        .expect("Failed to create dump_types table");

        // Insert a couple of rows. For SQLite, BOOLEAN is typically stored as 0/1
        // and NULL is allowed for the optional columns.
        db.execute_raw(Statement::from_string(
            backend,
            format!(
                "INSERT INTO {table_name} (active, counter, rating, name, created_at, uuid_col, json_col, opt_text, opt_int, opt_bool, array_col) VALUES
                    (0, 10, 3.14, 'foo', '2025-01-01T10:00:00Z', '11111111-1111-1111-1111-111111111111', '{{\"k\": \"v\"}}', NULL, NULL, NULL, '[1, 2, 3]'),
                    (1, 20, 6.28, 'bar', '2025-01-02T11:30:00Z', '22222222-2222-2222-2222-222222222222', '{{\"n\": 42}}', 'opt', 99, 1, '[\"a\", \"b\"]');"
            ),
        ))
        .await
        .expect("Failed to insert test data into dump_types table");

        // Act: dump the table into a YAML file in the temp tree_fs folder.
        let dump_dir = tree_fs.root.join("dump");
        std::fs::create_dir_all(&dump_dir).expect("Failed to create dump directory");

        dump_tables(&db, dump_dir.as_path(), Some(vec![table_name.to_string()]))
            .await
            .expect("dump_tables failed");

        let yaml_path = dump_dir.join(format!("{table_name}.yaml"));
        let yaml_content = std::fs::read_to_string(&yaml_path)
            .unwrap_or_else(|e| panic!("Failed to read YAML dump at {yaml_path:?}: {e}"));

        // Snapshot the actual YAML file contents, exactly as written by dump_tables.
        assert_snapshot!("dump_tables_sqlite_all_types", yaml_content);

        // Round-trip validation:
        // 1) Truncate the table
        db.execute_raw(Statement::from_string(
            backend,
            format!("DELETE FROM {table_name};"),
        ))
        .await
        .expect("Failed to truncate dump_types table");

        // 2) Seed it back from the dumped YAML using the same seed() logic
        crate::db::seed::<DumpTypesActiveModel>(
            &db,
            yaml_path.to_str().expect("YAML path should be valid UTF-8"),
        )
        .await
        .expect("seed from dumped YAML failed");

        // 3) Select rows back in a deterministic order and snapshot their JSON form.
        let models = DumpTypesEntity::find()
            .order_by_asc(dump_types_entity::Column::Id)
            .all(&db)
            .await
            .expect("select after seed failed");

        let roundtripped: Vec<serde_json::Value> = models
            .into_iter()
            .map(|m| serde_json::to_value(m).expect("serialize model"))
            .collect();

        assert_snapshot!(
            "dump_tables_sqlite_all_types_roundtrip",
            serde_json::to_string_pretty(&roundtripped).unwrap()
        );
    }
}
