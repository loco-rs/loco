use std::{
    collections::{BTreeMap, HashSet},
    fs,
    fs::File,
    io::Write,
    path::Path,
};

use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbBackend, DbErr, Statement};
use serde_json::json;

use super::IGNORED_TABLES;
use crate::{app::AppContext, Result as AppResult};

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

/// Normalize a SQLite-native datetime string to RFC3339.
///
/// `SQLite` has no native datetime type; Loco's `timestamptz` columns default to
/// `CURRENT_TIMESTAMP`, which `SQLite` writes as `"YYYY-MM-DD HH:MM:SS"` (space
/// separator, optional fractional seconds, no offset). Left verbatim in a dump,
/// that text fails chrono's RFC3339 parse when re-seeded into a typed
/// `DateTimeWithTimeZone` model (`Json("premature end of input")`, #1736). This
/// recognizes that space-separated form — interpreting a missing offset as UTC,
/// matching `SQLite`'s `CURRENT_TIMESTAMP` — and returns its RFC3339 rendering.
///
/// Returns `None` for input already in RFC3339 (left untouched, so no dump
/// churn) and for anything that is not a datetime at all (`parse_from_str`
/// requires the whole string to match, so ids, uuids and free text fall
/// through).
fn normalize_sqlite_datetime(s: &str) -> Option<String> {
    // Already RFC3339 (has `T` plus offset/`Z`): re-seeds fine, leave as-is.
    if DateTime::parse_from_rfc3339(s).is_ok() {
        return None;
    }
    // Space-separated but carrying an explicit offset.
    for fmt in [
        "%Y-%m-%d %H:%M:%S%.f%:z",
        "%Y-%m-%d %H:%M:%S%:z",
        "%Y-%m-%d %H:%M:%S%.f %:z",
        "%Y-%m-%d %H:%M:%S %:z",
    ] {
        if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
            return Some(dt.to_rfc3339());
        }
    }
    // Naive `YYYY-MM-DD HH:MM:SS[.f]`, interpreted as UTC (SQLite CURRENT_TIMESTAMP).
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc).to_rfc3339());
        }
    }
    None
}

/// Dumps the contents of specified database tables into YAML files.
///
/// This function retrieves data from all tables in the database, filters them
/// if `only_tables` is provided, and writes each table's content to a separate
/// YAML file in the specified directory.
///
/// # Errors
///
/// Returns an error if the operation fails for any reason or could not save the
/// content into a file.
// A linear dump routine: schema probe, per-table stream, and file write read
// most clearly as one top-to-bottom procedure.
#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
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
                    // Postgres decodes its native BOOLEAN as a distinct wire type that
                    // does not fall back to the numeric arms below (unlike SQLite,
                    // where booleans are stored as an INTEGER and already decode via
                    // the i8 arm), so without this arm PG boolean columns would be
                    // silently dropped from the dump entirely. Gate this on
                    // `boolean_columns` (rather than trying it unconditionally) because
                    // SQLite is dynamically typed and would happily decode any
                    // INTEGER column (e.g. `counter`, `id`) as a bool too.
                    .or_else(|_| {
                        if boolean_columns.contains(&col_name) {
                            row.try_get::<bool>("", &col_name)
                                .map(serde_json::Value::Bool)
                        } else {
                            Err(DbErr::Custom(format!(
                                "`{col_name}` is not a boolean column"
                            )))
                        }
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

                    // Normalize SQLite's space-separated datetime text to RFC3339 so
                    // the dump re-seeds cleanly into typed DateTimeWithTimeZone models
                    // (#1736). No-op for RFC3339 and non-datetime strings.
                    if let serde_json::Value::String(s) = &value
                        && let Some(normalized) = normalize_sqlite_datetime(s)
                    {
                        value = serde_json::Value::String(normalized);
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
    use sea_orm::EntityTrait;

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

    // Minimal entity with a real `DateTimeWithTimeZone` column, used to
    // reproduce #1736: a datetime dumped by the generic `dump_tables` must
    // re-seed cleanly.
    mod dt_entity {
        use sea_orm::entity::prelude::*;

        #[derive(
            Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize,
        )]
        #[sea_orm(table_name = "dt_reseed")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub created_at: DateTimeWithTimeZone,
        }

        #[derive(Copy, Clone, Debug, EnumIter)]
        pub enum Relation {}

        impl RelationTrait for Relation {
            fn def(&self) -> RelationDef {
                panic!("no relations for dt_entity::Relation")
            }
        }

        impl ActiveModelBehavior for ActiveModel {}
    }

    /// Regression for #1736: a `DateTimeWithTimeZone` column dumped through the
    /// generic `dump_tables` must re-seed cleanly. Loco's `timestamps_tz`
    /// columns carry `DEFAULT CURRENT_TIMESTAMP`, so SQLite fills them with its
    /// own `"YYYY-MM-DD HH:MM:SS"` text (space separator, no `T`, no offset).
    /// The dump captured that string verbatim (the string probe ran before the
    /// datetime probe), and re-seeding it into the typed `Model` then failed
    /// chrono's RFC3339 parse with `Json("premature end of input")`.
    #[tokio::test]
    async fn sqlite_dump_datetime_reseedable_regression_1736() {
        use crate::tests_cfg::config::get_sqlite_test_config;

        let (config, tree_fs) = get_sqlite_test_config("dt_reseed");
        let db = crate::db::connect(&config).await.expect("connect sqlite");
        let backend = db.get_database_backend();
        assert_eq!(backend, DatabaseBackend::Sqlite);

        // Model the real failing column exactly: a timestamptz-backed column
        // whose value comes from SQLite's `CURRENT_TIMESTAMP` default.
        db.execute_raw(Statement::from_string(
            backend,
            "CREATE TABLE dt_reseed (id INTEGER PRIMARY KEY AUTOINCREMENT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);".to_string(),
        ))
        .await
        .expect("create dt_reseed table");

        // Insert letting the DEFAULT fill created_at with the SQLite-native
        // `"YYYY-MM-DD HH:MM:SS"` form — the exact bytes a real app stores.
        db.execute_raw(Statement::from_string(
            backend,
            "INSERT INTO dt_reseed (created_at) VALUES ('2026-01-31 19:34:29');".to_string(),
        ))
        .await
        .expect("insert row");

        let dump_dir = tree_fs.root.join("dump");
        std::fs::create_dir_all(&dump_dir).expect("create dump dir");
        dump_tables(&db, dump_dir.as_path(), Some(vec!["dt_reseed".to_string()]))
            .await
            .expect("dump_tables failed");
        let yaml_path = dump_dir.join("dt_reseed.yaml");

        db.execute_raw(Statement::from_string(
            backend,
            "DELETE FROM dt_reseed;".to_string(),
        ))
        .await
        .expect("truncate dt_reseed");

        crate::db::seed::<dt_entity::ActiveModel>(&db, yaml_path.to_str().expect("utf8 path"))
            .await
            .expect("re-seed of dumped datetime must succeed (#1736)");

        let rows = dt_entity::Entity::find()
            .all(&db)
            .await
            .expect("select after reseed");
        assert_eq!(rows.len(), 1, "exactly one row should be re-seeded");
        // The instant must be preserved (SQLite CURRENT_TIMESTAMP is UTC).
        assert_eq!(
            rows[0].created_at.to_utc(),
            DateTime::parse_from_rfc3339("2026-01-31T19:34:29+00:00")
                .expect("parse expected")
                .to_utc(),
            "the re-seeded datetime must equal the original instant"
        );
    }

    #[tokio::test]
    async fn postgres_dump_tables_boolean_column() {
        use crate::tests_cfg::postgres::setup_postgres_container;

        // Arrange: spin up a real Postgres container and create a table with a
        // native BOOLEAN column. This reproduces the defect where PG BOOLEAN
        // values are silently dropped from dump_tables because the decode
        // probe chain had no `bool` arm (PG bools don't fall back to the
        // numeric arms the way SQLite's integer-backed booleans do).
        let (pg_url, _container) = setup_postgres_container().await;

        let mut config = crate::tests_cfg::config::get_database_config();
        config.uri = pg_url;
        config.min_connections = 1;
        config.max_connections = 5;

        let db = crate::db::connect(&config)
            .await
            .expect("Failed to connect to PostgreSQL test database");
        let backend = db.get_database_backend();
        assert_eq!(backend, DatabaseBackend::Postgres);

        let table_name = "dump_bool_types";

        db.execute_raw(Statement::from_string(
            backend,
            format!(
                "CREATE TABLE {table_name} (
                    id SERIAL PRIMARY KEY,
                    active BOOLEAN NOT NULL,
                    name TEXT NOT NULL
                );"
            ),
        ))
        .await
        .expect("Failed to create dump_bool_types table");

        db.execute_raw(Statement::from_string(
            backend,
            format!(
                "INSERT INTO {table_name} (active, name) VALUES
                    (true, 'foo'),
                    (false, 'bar');"
            ),
        ))
        .await
        .expect("Failed to insert test data into dump_bool_types table");

        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp folder");
        let dump_dir = tree_fs.root.join("dump");
        std::fs::create_dir_all(&dump_dir).expect("Failed to create dump directory");

        dump_tables(&db, dump_dir.as_path(), Some(vec![table_name.to_string()]))
            .await
            .expect("dump_tables failed");

        let yaml_path = dump_dir.join(format!("{table_name}.yaml"));
        let yaml_content = std::fs::read_to_string(&yaml_path)
            .unwrap_or_else(|e| panic!("Failed to read YAML dump at {yaml_path:?}: {e}"));

        let parsed: Vec<BTreeMap<String, serde_json::Value>> =
            serde_yaml::from_str(&yaml_content).expect("dumped YAML should parse");

        assert_eq!(parsed.len(), 2, "expected two dumped rows");

        // The `active` column must be present (not dropped) and decoded as a
        // real JSON boolean, not a number or missing entirely.
        let foo_row = parsed
            .iter()
            .find(|row| row.get("name") == Some(&serde_json::Value::String("foo".to_string())))
            .expect("row with name=foo should be present in the dump");
        assert_eq!(
            foo_row.get("active"),
            Some(&serde_json::Value::Bool(true)),
            "boolean column 'active' should be dumped as `true`, got: {foo_row:?}"
        );

        let bar_row = parsed
            .iter()
            .find(|row| row.get("name") == Some(&serde_json::Value::String("bar".to_string())))
            .expect("row with name=bar should be present in the dump");
        assert_eq!(
            bar_row.get("active"),
            Some(&serde_json::Value::Bool(false)),
            "boolean column 'active' should be dumped as `false`, got: {bar_row:?}"
        );
    }
}
