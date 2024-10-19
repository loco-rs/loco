use async_trait::async_trait;
use axum::{Extension, Router as AxumRouter};
use loco_rs::prelude::*;
use mongodb::{bson::doc, options::ClientOptions, Client, Database};

#[allow(clippy::module_name_repetitions)]
pub struct MongoDbInitializer;

#[async_trait]
impl Initializer for MongoDbInitializer {
    fn name(&self) -> String {
        "mongodb".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let mongo_db_config = ctx
            .config
            .initializers
            .clone()
            .ok_or_else(|| Error::Message("initializers config not configured".to_string()))?;

        let mongo_db_value = mongo_db_config
            .get("mongodb")
            .ok_or_else(|| Error::Message("mongo not configured as initializer".to_string()))?;

        let mongo_db: MongoDbConfig = serde_json::from_value(mongo_db_value.clone())
            .map_err(|e| Error::Message(e.to_string()))?;

        let db = connect_to_db(mongo_db).await?;

        Ok(router.layer(Extension(db)))
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct MongoDbConfig {
    uri: String,
    db_name: String,
    client_options: Option<ClientOptions>,
}

async fn connect_to_db(config: MongoDbConfig) -> Result<Database> {
    let mut client_options = ClientOptions::parse_async(&config.uri)
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

    let client_options = merge_config_with_client(&mut client_options, config.clone());

    let client = Client::with_options(client_options).map_err(|e| Error::Message(e.to_string()))?;

    let db = client.database(config.db_name.as_ref());

    // Ping the Database to make sure a connection has been made
    db.run_command(doc! { "ping": 1 }, None)
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

    Ok(db)
}

fn merge_config_with_client(co: &mut ClientOptions, config: MongoDbConfig) -> ClientOptions {
    match config.client_options {
        None => co.clone(),
        Some(client_options) => {
            co.app_name = client_options.app_name.or(co.app_name.clone());
            co.compressors = client_options.compressors.or(co.compressors.clone());
            co.cmap_event_handler = client_options
                .cmap_event_handler
                .or(co.cmap_event_handler.clone());
            co.command_event_handler = client_options
                .command_event_handler
                .or(co.command_event_handler.clone());
            co.connect_timeout = client_options.connect_timeout.or(co.connect_timeout);
            co.credential = client_options.credential.or(co.credential.clone());
            co.direct_connection = client_options.direct_connection.or(co.direct_connection);
            co.driver_info = client_options.driver_info.or(co.driver_info.clone());
            co.heartbeat_freq = client_options.heartbeat_freq.or(co.heartbeat_freq);
            co.load_balanced = client_options.load_balanced.or(co.load_balanced);
            co.local_threshold = client_options.local_threshold.or(co.local_threshold);
            co.max_idle_time = client_options.max_idle_time.or(co.max_idle_time);
            co.max_pool_size = client_options.max_pool_size.or(co.max_pool_size);
            co.min_pool_size = client_options.min_pool_size.or(co.min_pool_size);
            co.max_connecting = client_options.max_connecting.or(co.max_connecting);
            co.read_concern = client_options.read_concern.or(co.read_concern.clone());
            co.repl_set_name = client_options.repl_set_name.or(co.repl_set_name.clone());
            co.retry_reads = client_options.retry_reads.or(co.retry_reads);
            co.retry_writes = client_options.retry_writes.or(co.retry_writes);
            co.sdam_event_handler = client_options
                .sdam_event_handler
                .or(co.sdam_event_handler.clone());
            co.selection_criteria = client_options
                .selection_criteria
                .or(co.selection_criteria.clone());
            co.server_api = client_options.server_api.or(co.server_api.clone());
            co.server_selection_timeout = client_options
                .server_selection_timeout
                .or(co.server_selection_timeout);
            co.default_database = client_options
                .default_database
                .or(co.default_database.clone());
            co.tls = client_options.tls.or(co.tls.clone());
            // co.tracing_max_document_length_bytes =
            // client_options.tracing_max_document_length_bytes;
            co.write_concern = client_options.write_concern.or(co.write_concern.clone());
            co.srv_max_hosts = client_options.srv_max_hosts.or(co.srv_max_hosts);

            co.clone()
        }
    }
}
