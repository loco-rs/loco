use url::Url;

use crate::{errors::Error, Result};

pub async fn discover_table_names(database_url: &str) -> Result<Vec<String>> {
    let database_schema = "public";
    let max_connections = 2;
    // The database should be a valid URL that can be parsed
    // protocol://username:password@host/database_name
    let url = Url::parse(database_url).map_err(Box::from)?;
    let is_sqlite = url.scheme() == "sqlite";

    let filter_hidden_tables = |table: &str| -> bool { !table.starts_with('_') };

    let database_name = if !is_sqlite {
        // The database name should be the first element of the path string
        //
        // Throwing an error if there is no database name since it might be
        // accepted by the database without it, while we're looking to dump
        // information from a particular database
        let database_name = url
            .path_segments()
            .unwrap_or_else(|| {
                panic!(
                    "There is no database name as part of the url path: {}",
                    url.as_str()
                )
            })
            .next()
            .unwrap();

        // An empty string as the database name is also an error
        if database_name.is_empty() {
            panic!(
                "There is no database name as part of the url path: {}",
                url.as_str()
            );
        }

        database_name
    } else {
        Default::default()
    };

    let names = match url.scheme() {
        "mysql" => {
            use sea_schema::mysql::discovery::SchemaDiscovery;
            use sqlx::MySql;
            let connection = connect::<MySql>(max_connections, url.as_str(), None).await?;
            let schema_discovery = SchemaDiscovery::new(connection, database_name);
            let schema = schema_discovery.discover().await.map_err(Box::from)?;
            let names: Vec<String> = schema
                .tables
                .into_iter()
                .filter(|schema| filter_hidden_tables(&schema.info.name))
                .map(|schema| schema.info.name.to_string())
                .collect();
            names
        }
        "sqlite" => {
            use sea_schema::sqlite::discovery::SchemaDiscovery;
            use sqlx::Sqlite;

            let connection = connect::<Sqlite>(max_connections, url.as_str(), None).await?;
            let schema_discovery = SchemaDiscovery::new(connection);
            let schema = schema_discovery.discover().await.map_err(Box::from)?;
            let names = schema
                .tables
                .into_iter()
                .filter(|schema| filter_hidden_tables(&schema.name))
                .map(|schema| schema.name.to_string())
                .collect();
            names
        }
        "postgres" | "postgresql" => {
            use sea_schema::postgres::discovery::SchemaDiscovery;
            use sqlx::Postgres;
            let schema = &database_schema;
            let connection = connect::<Postgres>(max_connections, url.as_str(), Some(schema))
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
            let schema_discovery = SchemaDiscovery::new(connection, schema);
            let schema = schema_discovery.discover().await.map_err(Box::from)?;
            let names = schema
                .tables
                .into_iter()
                .filter(|schema| filter_hidden_tables(&schema.info.name))
                .map(|schema| schema.info.name.to_string())
                .collect();
            names
        }
        _ => unimplemented!("{} is not supported", url.scheme()),
    };
    Ok(names)
}

async fn connect<DB>(
    max_connections: u32,
    url: &str,
    schema: Option<&str>,
) -> Result<sqlx::Pool<DB>>
where
    DB: sqlx::Database,
    for<'a> &'a mut <DB as sqlx::Database>::Connection: sqlx::Executor<'a>,
{
    let mut pool_options = sqlx::pool::PoolOptions::<DB>::new().max_connections(max_connections);
    // Set search_path for Postgres, E.g. Some("public") by default
    // MySQL & SQLite connection initialize with schema `None`
    if let Some(schema) = schema {
        let sql = format!("SET search_path = '{schema}'");
        pool_options = pool_options.after_connect(move |conn, _| {
            let sql = sql.clone();
            Box::pin(async move {
                sqlx::Executor::execute(conn, sql.as_str())
                    .await
                    .map(|_| ())
            })
        });
    }
    Ok(pool_options.connect(url).await.map_err(Box::from)?)
}
