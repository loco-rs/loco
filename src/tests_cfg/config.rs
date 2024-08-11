use crate::{
    config::{self, Config},
    logger,
};

#[must_use]
pub fn test_config() -> Config {
    Config {
        logger: config::Logger {
            enable: false,
            pretty_backtrace: true,
            level: logger::LogLevel::Off,
            format: logger::Format::Json,
            override_filter: None,
            file_appender: None,
        },
        server: config::Server {
            binding: "localhost".to_string(),
            port: 3000,
            host: "localhost".to_string(),
            ident: None,
            middlewares: config::Middlewares {
                compression: None,
                etag: None,
                limit_payload: None,
                logger: None,
                catch_panic: None,
                timeout_request: None,
                cors: None,
                request_context: None,
                static_assets: None,
            },
        },
        #[cfg(feature = "with-db")]
        database: config::Database {
            uri: "sqlite::memory:".to_string(),
            enable_logging: false,
            min_connections: 1,
            max_connections: 1,
            connect_timeout: 1,
            idle_timeout: 1,
            acquire_timeout: None,
            auto_migrate: false,
            dangerously_truncate: false,
            dangerously_recreate: false,
        },
        queue: None,
        auth: None,
        workers: config::Workers {
            mode: config::WorkerMode::ForegroundBlocking,
            queues: None,
        },
        mailer: None,
        initializers: None,
        settings: None,
    }
}
