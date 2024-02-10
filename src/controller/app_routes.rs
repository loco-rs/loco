//! This module defines the [`AppRoutes`] struct that is responsible for
//! configuring routes in an Axum application. It allows you to define route
//! prefixes, add routes, and configure middlewares for the application.

use std::{path::PathBuf, time::Duration};

use axum::{http, response::IntoResponse, Router as AXRouter};
use lazy_static::lazy_static;
use regex::Regex;
use tower_http::{
    add_extension::AddExtensionLayer,
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

#[cfg(feature = "channels")]
use super::channels::AppChannels;
use super::routes::Routes;
use crate::{
    app::AppContext, config, controller::middleware::etag::EtagLayer, environment::Environment,
    errors, Result,
};

lazy_static! {
    static ref NORMALIZE_URL: Regex = Regex::new(r"/+").unwrap();
    static ref DEFAULT_IDENT_HEADER_NAME: http::header::HeaderName =
        http::header::HeaderName::from_static("x-powered-by");
    static ref DEFAULT_IDENT_HEADER_VALUE: http::header::HeaderValue =
        http::header::HeaderValue::from_static("loco.rs");
}

/// Represents the routes of the application.
#[derive(Clone)]
pub struct AppRoutes {
    prefix: Option<String>,
    routes: Vec<Routes>,
    #[cfg(feature = "channels")]
    channels: Option<AppChannels>,
}

pub struct ListRoutes {
    pub uri: String,
    pub actions: Vec<axum::http::Method>,
    pub method: axum::routing::MethodRouter<AppContext>,
}

impl ToString for ListRoutes {
    fn to_string(&self) -> String {
        let actions_str = self
            .actions
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        // Define your custom logic here to format the struct as a string
        format!("[{}] {}", actions_str, self.uri)
    }
}

impl AppRoutes {
    /// Create a new instance with the default routes.
    #[must_use]
    pub fn with_default_routes() -> Self {
        let routes = Self::empty().add_route(super::ping::routes());
        #[cfg(feature = "with-db")]
        let routes = routes.add_route(super::health::routes());

        routes
    }

    /// Create an empty instance.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            prefix: None,
            routes: vec![],
            #[cfg(feature = "channels")]
            channels: None,
        }
    }

    #[must_use]
    pub fn collect(&self) -> Vec<ListRoutes> {
        let base_url_prefix = self.get_prefix().map_or("/", |url| url.as_str());

        self.get_routes()
            .iter()
            .flat_map(|router| {
                let mut uri_parts = vec![base_url_prefix];
                if let Some(prefix) = router.prefix.as_ref() {
                    uri_parts.push(prefix);
                }
                router.handlers.iter().map(move |controller| {
                    let uri = format!("{}{}", uri_parts.join("/"), &controller.uri);
                    let binding = NORMALIZE_URL.replace_all(&uri, "/");

                    let uri = if binding.len() > 1 {
                        NORMALIZE_URL
                            .replace_all(&uri, "/")
                            .strip_suffix('/')
                            .map_or_else(|| binding.to_string(), std::string::ToString::to_string)
                    } else {
                        binding.to_string()
                    };

                    ListRoutes {
                        uri,
                        actions: controller.actions.clone(),
                        method: controller.method.clone(),
                    }
                })
            })
            .collect()
    }

    /// Get the prefix of the routes.
    #[must_use]
    pub fn get_prefix(&self) -> Option<&String> {
        self.prefix.as_ref()
    }

    /// Get the routes.
    #[must_use]
    pub fn get_routes(&self) -> &[Routes] {
        self.routes.as_ref()
    }

    /// Set a prefix for the routes. this prefix will be a prefix for all the
    /// routes.
    ///
    /// # Example
    ///
    /// In the following example you are adding api as a prefix for all routes
    ///
    /// ```rust
    /// use loco_rs::controller::AppRoutes;
    ///
    /// AppRoutes::with_default_routes().prefix("api");
    /// ```
    #[must_use]
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    /// Add a single route.
    #[must_use]
    pub fn add_route(mut self, route: Routes) -> Self {
        self.routes.push(route);
        self
    }

    /// Add multiple routes.
    #[must_use]
    pub fn add_routes(mut self, mounts: Vec<Routes>) -> Self {
        for mount in mounts {
            self.routes.push(mount);
        }
        self
    }

    #[cfg(feature = "channels")]
    #[must_use]
    pub fn add_app_channels(mut self, channels: AppChannels) -> Self {
        self.channels = Some(channels);
        self
    }

    /// Convert the routes to an Axum Router, and set a list of middlewares that
    /// configure in the [`config::Config`]
    ///
    /// # Errors
    /// Return an [`Result`] when could not convert the router setup to
    /// [`axum::Router`].
    #[allow(clippy::cognitive_complexity)]
    pub fn to_router(&self, ctx: AppContext) -> Result<AXRouter> {
        let mut app = AXRouter::new();

        for router in self.collect() {
            tracing::info!("{}", router.to_string());

            app = app.route(&router.uri, router.method);
        }

        app = Self::add_powered_by_header(app, &ctx.config.server);

        let middlewares = &ctx.config.server.middlewares;

        if middlewares
            .catch_panic
            .as_ref()
            .is_some_and(|cp| cp.is_enabled())
        {
            app = Self::add_catch_panic(app);
        }

        if middlewares
            .compression
            .as_ref()
            .is_some_and(|c| c.is_enabled())
        {
            app = Self::add_compression_middleware(app);
        }

        if let Some(limit) = middlewares
            .limit_payload
            .as_ref()
            .and_then(|limit_payload| limit_payload.as_ref())
        {
            app = Self::add_limit_payload_middleware(app, limit)?;
        }

        if middlewares.logger.as_ref().is_some_and(|l| l.is_enabled()) {
            app = Self::add_logger_middleware(app, &ctx.environment);
        }

        if let Some(timeout_request) = middlewares
            .timeout_request
            .as_ref()
            .and_then(|t| t.as_ref())
        {
            app = Self::add_timeout_middleware(app, timeout_request);
        }

        if let Some(cors) = middlewares
            .cors
            .as_ref()
            .and_then(|c| c.as_ref())
            .map(Self::get_cors_middleware)
            .transpose()?
        {
            app = app.layer(cors.clone());
            tracing::info!("[Middleware] Adding cors");
        }

        if let Some(static_assets) = middlewares.static_assets.as_ref().and_then(|e| e.as_ref()) {
            app = Self::add_static_asset_middleware(app, static_assets)?;
        }

        if middlewares.etag.as_ref().is_some_and(|e| e.is_enabled()) {
            app = Self::add_etag_middleware(app);
        }

        #[cfg(feature = "channels")]
        if let Some(channels) = self.channels.as_ref() {
            tracing::info!("[Middleware] Adding channels");
            let channel_layer_app = tower::ServiceBuilder::new().layer(channels.layer.clone());

            app = app.layer(
                tower::ServiceBuilder::new()
                    .layer(tower_http::cors::CorsLayer::permissive())
                    .layer(channels.layer.clone()),
            );
        }

        let router = app.with_state(ctx);
        Ok(router)
    }

    fn add_static_asset_middleware(
        app: AXRouter<AppContext>,
        config: &config::StaticAssetsMiddleware,
    ) -> Result<AXRouter<AppContext>> {
        if config.must_exist
            && (!PathBuf::from(&config.folder.path).exists()
                || !PathBuf::from(&config.fallback).exists())
        {
            return Err(errors::Error::Message(format!(
                "one of the static path are not found, Folder `{}` fallback: `{}`",
                config.folder.path, config.fallback,
            )));
        }

        tracing::info!("[Middleware] Adding static");
        let serve_dir =
            ServeDir::new(&config.folder.path).not_found_service(ServeFile::new(&config.fallback));
        Ok(app.nest_service(
            &config.folder.uri,
            if config.precompressed {
                tracing::info!("[Middleware] Enable precompressed static assets");
                serve_dir.precompressed_gzip()
            } else {
                serve_dir
            },
        ))
    }

    fn add_compression_middleware(app: AXRouter<AppContext>) -> AXRouter<AppContext> {
        let app = app.layer(CompressionLayer::new());
        tracing::info!("[Middleware] Adding compression layer");
        app
    }

    fn add_etag_middleware(app: AXRouter<AppContext>) -> AXRouter<AppContext> {
        let app = app.layer(EtagLayer::new());
        tracing::info!("[Middleware] Adding etag layer");
        app
    }

    fn get_cors_middleware(config: &config::CorsMiddleware) -> Result<cors::CorsLayer> {
        let mut cors: cors::CorsLayer = cors::CorsLayer::permissive();

        if let Some(allow_origins) = &config.allow_origins {
            // testing CORS, assuming https://example.com in the allow list:
            // $ curl -v --request OPTIONS 'localhost:3000/api/_ping' -H 'Origin: https://example.com' -H 'Access-Control-Request-Method: GET'
            // look for '< access-control-allow-origin: https://example.com' in response.
            // if it doesn't appear (test with a bogus domain), it is not allowed.
            let mut list = vec![];
            for origins in allow_origins {
                list.push(origins.parse()?);
            }
            cors = cors.allow_origin(list);
        }

        if let Some(allow_headers) = &config.allow_headers {
            let mut headers = vec![];
            for header in allow_headers {
                headers.push(header.parse()?);
            }
            cors = cors.allow_headers(headers);
        }

        if let Some(allow_methods) = &config.allow_methods {
            let mut methods = vec![];
            for method in allow_methods {
                methods.push(method.parse()?);
            }
            cors = cors.allow_methods(methods);
        }

        if let Some(max_age) = config.max_age {
            cors = cors.max_age(Duration::from_secs(max_age));
        }

        Ok(cors)
    }

    fn add_catch_panic(app: AXRouter<AppContext>) -> AXRouter<AppContext> {
        app.layer(CatchPanicLayer::custom(handle_panic))
    }

    fn add_limit_payload_middleware(
        app: AXRouter<AppContext>,
        limit: &config::LimitPayloadMiddleware,
    ) -> Result<AXRouter<AppContext>> {
        let app = app.layer(axum::extract::DefaultBodyLimit::max(
            byte_unit::Byte::from_str(&limit.body_limit)
                .map_err(Box::from)?
                .get_bytes() as usize,
        ));
        tracing::info!(
            data = &limit.body_limit,
            "[Middleware] Adding limit payload",
        );

        Ok(app)
    }
    fn add_logger_middleware(
        app: AXRouter<AppContext>,
        environment: &Environment,
    ) -> AXRouter<AppContext> {
        let app = app
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &http::Request<_>| {
                    let request_id = uuid::Uuid::new_v4();
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
            .layer(AddExtensionLayer::new(environment.clone()));

        tracing::info!("[Middleware] Adding log trace id",);
        app
    }

    fn add_timeout_middleware(
        app: AXRouter<AppContext>,
        config: &config::TimeoutRequestMiddleware,
    ) -> AXRouter<AppContext> {
        let app = app.layer(TimeoutLayer::new(Duration::from_millis(config.timeout)));

        tracing::info!("[Middleware] Adding timeout layer");
        app
    }

    fn add_powered_by_header(
        app: AXRouter<AppContext>,
        config: &config::Server,
    ) -> AXRouter<AppContext> {
        let ident_value = config.ident.as_ref().map_or_else(
            || Some(DEFAULT_IDENT_HEADER_VALUE.clone()),
            |ident| {
                if ident.is_empty() {
                    None
                } else {
                    match http::header::HeaderValue::from_str(ident) {
                        Ok(val) => Some(val),
                        Err(e) => {
                            tracing::info!(
                                error = format!("{}", e),
                                val = ident,
                                "could not set custom ident header"
                            );
                            Some(DEFAULT_IDENT_HEADER_VALUE.clone())
                        }
                    }
                }
            },
        );

        if let Some(value) = ident_value {
            app.layer(SetResponseHeaderLayer::overriding(
                DEFAULT_IDENT_HEADER_NAME.clone(),
                value,
            ))
        } else {
            app
        }
    }
}

/// Handler function for the [`CatchPanicLayer`] middleware.
#[allow(clippy::needless_pass_by_value)]
fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> axum::response::Response {
    let err = err.downcast_ref::<String>().map_or_else(
        || err.downcast_ref::<&str>().map_or("no error details", |s| s),
        |s| s.as_str(),
    );

    tracing::error!(err = err, "server get panic");

    errors::Error::InternalServerError.into_response()
}
