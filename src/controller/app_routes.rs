//! This module defines the [`AppRoutes`] struct that is responsible for
//! configuring routes in an Axum application. It allows you to define route
//! prefixes, add routes, and configure middlewares for the application.

use std::{fmt, path::PathBuf, time::Duration};

use axum::{http, response::IntoResponse, Router as AXRouter};
use lazy_static::lazy_static;
use regex::Regex;
use tower_http::{
    add_extension::AddExtensionLayer,
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

#[cfg(feature = "channels")]
use super::channels::AppChannels;
use super::{
    middleware::{cors::cors_middleware, remote_ip::RemoteIPConfig, secure_headers::SecureHeaders},
    routes::Routes,
};
use crate::{
    app::AppContext,
    config,
    controller::middleware::{
        etag::EtagLayer,
        remote_ip::RemoteIPLayer,
        request_id::{request_id_middleware, LocoRequestId},
    },
    environment::Environment,
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

impl fmt::Display for ListRoutes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let actions_str = self
            .actions
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");

        write!(f, "[{}] {}", actions_str, self.uri)
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

    /// Add the routes to an existing Axum Router, and set a list of middlewares
    /// that configure in the [`config::Config`]
    ///
    /// # Errors
    /// Return an [`Result`] when could not convert the router setup to
    /// [`axum::Router`].
    #[allow(clippy::cognitive_complexity)]
    pub fn to_router(&self, ctx: AppContext, mut app: AXRouter<AppContext>) -> Result<AXRouter> {
        //
        // IMPORTANT: middleware ordering in this function is opposite to what you
        // intuitively may think. when using `app.layer` to add individual middleware,
        // the LAST middleware is the FIRST to meet the outside world (a user request
        // starting), or "LIFO" order.
        // We build the "onion" from the inside (start of this function),
        // outwards (end of this function). This is why routes is first in coding order
        // here (the core of the onion), and request ID is amongst the last
        // (because every request is assigned with a unique ID, which starts its
        // "life").
        //
        // NOTE: when using ServiceBuilder#layer the order is FIRST to LAST (but we
        // don't use ServiceBuilder because it requires too complex generic typing for
        // this function). ServiceBuilder is recommended to save compile times, but that
        // may be a thing of the past as we don't notice any issues with compile times
        // using the router directly, and ServiceBuilder has been reported to give
        // issues in compile times itself (https://github.com/rust-lang/crates.io/pull/7443).
        //
        for router in self.collect() {
            tracing::info!("{}", router.to_string());

            app = app.route(&router.uri, router.method);
        }

        #[cfg(feature = "channels")]
        if let Some(channels) = self.channels.as_ref() {
            tracing::info!("[Middleware] +channels");
            let channel_layer_app = tower::ServiceBuilder::new().layer(channels.layer.clone());
            if let Some(cors) = &ctx
                .config
                .server
                .middlewares
                .cors
                .as_ref()
                .filter(|c| c.enable)
            {
                app = app.layer(
                    tower::ServiceBuilder::new()
                        .layer(cors_middleware(cors)?)
                        .layer(channel_layer_app),
                );
            } else {
                app = app.layer(
                    tower::ServiceBuilder::new()
                        .layer(tower_http::cors::CorsLayer::permissive())
                        .layer(channel_layer_app),
                );
            }
        }

        if let Some(catch_panic) = &ctx.config.server.middlewares.catch_panic {
            if catch_panic.enable {
                app = Self::add_catch_panic(app);
            }
        }

        if let Some(etag) = &ctx.config.server.middlewares.etag {
            if etag.enable {
                app = Self::add_etag_middleware(app);
            }
        }

        if let Some(remote_ip) = &ctx.config.server.middlewares.remote_ip {
            if remote_ip.enable {
                app = Self::add_remote_ip_middleware(app, remote_ip)?;
            }
        }

        if let Some(compression) = &ctx.config.server.middlewares.compression {
            if compression.enable {
                app = Self::add_compression_middleware(app);
            }
        }

        if let Some(timeout_request) = &ctx.config.server.middlewares.timeout_request {
            if timeout_request.enable {
                app = Self::add_timeout_middleware(app, timeout_request);
            }
        }

        if let Some(cors) = &ctx.config.server.middlewares.cors {
            if cors.enable {
                app = app.layer(cors_middleware(cors)?);
            }
        }

        if let Some(limit) = &ctx.config.server.middlewares.limit_payload {
            if limit.enable {
                app = Self::add_limit_payload_middleware(app, limit)?;
            }
        }

        if let Some(logger) = &ctx.config.server.middlewares.logger {
            if logger.enable {
                app = Self::add_logger_middleware(app, &ctx.environment);
            }
        }

        if let Some(static_assets) = &ctx.config.server.middlewares.static_assets {
            if static_assets.enable {
                app = Self::add_static_asset_middleware(app, static_assets)?;
            }
        }

        if let Some(secure_headers) = &ctx.config.server.middlewares.secure_headers {
            app = app.layer(SecureHeaders::new(secure_headers)?);
            tracing::info!("[Middleware] +secure headers");
        }

        app = Self::add_powered_by_header(app, &ctx.config.server);

        app = Self::add_request_id_middleware(app);

        let router = app.with_state(ctx);
        Ok(router)
    }

    fn add_request_id_middleware(app: AXRouter<AppContext>) -> AXRouter<AppContext> {
        let app = app.layer(axum::middleware::from_fn(request_id_middleware));
        tracing::info!("[Middleware] +request id");
        app
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

        tracing::info!("[Middleware] +static assets");
        let serve_dir =
            ServeDir::new(&config.folder.path).not_found_service(ServeFile::new(&config.fallback));
        Ok(app.nest_service(
            &config.folder.uri,
            if config.precompressed {
                tracing::info!("[Middleware] +precompressed static assets");
                serve_dir.precompressed_gzip()
            } else {
                serve_dir
            },
        ))
    }

    fn add_compression_middleware(app: AXRouter<AppContext>) -> AXRouter<AppContext> {
        let app = app.layer(CompressionLayer::new());
        tracing::info!("[Middleware] +compression");
        app
    }

    fn add_etag_middleware(app: AXRouter<AppContext>) -> AXRouter<AppContext> {
        let app = app.layer(EtagLayer::new());
        tracing::info!("[Middleware] +etag");
        app
    }

    fn add_remote_ip_middleware(
        app: AXRouter<AppContext>,
        config: &RemoteIPConfig,
    ) -> Result<AXRouter<AppContext>> {
        let app = app.layer(RemoteIPLayer::new(config)?);
        tracing::info!("[Middleware] +remote IP");
        Ok(app)
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
        tracing::info!(data = &limit.body_limit, "[Middleware] +limit payload",);

        Ok(app)
    }
    fn add_logger_middleware(
        app: AXRouter<AppContext>,
        environment: &Environment,
    ) -> AXRouter<AppContext> {
        let app = app
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
            .layer(AddExtensionLayer::new(environment.clone()));

        tracing::info!("[Middleware] +log trace id",);
        app
    }

    fn add_timeout_middleware(
        app: AXRouter<AppContext>,
        config: &config::TimeoutRequestMiddleware,
    ) -> AXRouter<AppContext> {
        let app = app.layer(TimeoutLayer::new(Duration::from_millis(config.timeout)));

        tracing::info!("[Middleware] +timeout");
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

    tracing::error!(err.msg = err, "server_panic");

    errors::Error::InternalServerError.into_response()
}
