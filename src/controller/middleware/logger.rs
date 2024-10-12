//! Logger Middleware
//!
//! This middleware provides logging functionality for HTTP requests. It uses `TraceLayer` to
//! log detailed information about each request, such as the HTTP method, URI, version, user agent,
//! and an associated request ID. Additionally, it integrates the application's runtime environment
//! into the log context, allowing environment-specific logging (e.g., "development", "production").

use crate::{
    app::AppContext,
    controller::middleware::{request_id::LocoRequestId, MiddlewareLayer},
    environment::Environment,
    Result,
};
use axum::{http, Router as AXRouter};
use serde::{Deserialize, Serialize};
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    enable: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { enable: true }
    }
}

/// [`Middleware`] struct responsible for logging HTTP requests.
#[derive(Serialize)]
pub struct Middleware {
    config: Config,
    environment: Environment,
}

/// Creates a new instance of [`Middleware`] by cloning the [`Config`] configuration.
#[must_use]
pub fn new(config: &Config, environment: &Environment) -> Middleware {
    Middleware {
        config: config.clone(),
        environment: environment.clone(),
    }
}

impl MiddlewareLayer for Middleware {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "logger"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.config.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the logger middleware to the application router by adding layers for:
    ///
    /// - `TraceLayer`: Logs detailed information about each HTTP request.
    /// - `AddExtensionLayer`: Adds the current environment to the request extensions, making it
    ///   accessible to the `TraceLayer` for logging.
    ///
    /// The `TraceLayer` is customized with `make_span_with` to extract request-specific details
    /// like method, URI, version, user agent, and request ID, then create a tracing span for the request.
    ///
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &http::Request<_>| {
                    let ext = request.extensions();
                    let request_id = ext
                        .get::<LocoRequestId>()
                        .map_or_else(|| "req-id-none".to_string(), |r| r.get().to_string());
                    let user_agent = request
                        .headers()
                        .get(axum::http::header::USER_AGENT)
                        .map_or("", |h| h.to_str().unwrap_or(""));

                    let env: String = request
                        .extensions()
                        .get::<Environment>()
                        .map(std::string::ToString::to_string)
                        .unwrap_or_default();

                    tracing::error_span!(
                        "http-request",
                        "http.method" = tracing::field::display(request.method()),
                        "http.uri" = tracing::field::display(request.uri()),
                        "http.version" = tracing::field::debug(request.version()),
                        "http.user_agent" = tracing::field::display(user_agent),
                        "environment" = tracing::field::display(env),
                        request_id = tracing::field::display(request_id),
                    )
                }),
            )
            .layer(AddExtensionLayer::new(self.environment.clone())))
    }
}
