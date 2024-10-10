//! This module defines the [`AppRoutes`] struct that is responsible for
//! configuring routes in an Axum application. It allows you to define route
//! prefixes, add routes, and configure middlewares for the application.

use std::fmt;

use axum::Router as AXRouter;
use lazy_static::lazy_static;
use regex::Regex;

#[cfg(feature = "channels")]
use super::channels::AppChannels;
use crate::{
    app::{AppContext, Hooks},
    controller::{middleware::MiddlewareLayer, routes::Routes},
    Result,
};

lazy_static! {
    static ref NORMALIZE_URL: Regex = Regex::new(r"/+").unwrap();
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
        let base_url_prefix = self
            .get_prefix()
            // add a leading slash forcefully. Axum routes must start with a leading slash.
            // if we have double leading slashes - it will get normalized into a single slash later
            .map_or("/".to_string(), |url| format!("/{}", url.as_str()));

        self.get_routes()
            .iter()
            .flat_map(|controller| {
                let mut uri_parts = vec![base_url_prefix.clone()];
                if let Some(prefix) = controller.prefix.as_ref() {
                    uri_parts.push(prefix.to_string());
                }
                controller.handlers.iter().map(move |handler| {
                    let mut parts = uri_parts.clone();
                    parts.push(handler.uri.to_string());
                    let joined_parts = parts.join("/");

                    let normalized = NORMALIZE_URL.replace_all(&joined_parts, "/");
                    let uri = if normalized == "/" {
                        normalized.to_string()
                    } else {
                        normalized.strip_suffix('/').map_or_else(
                            || normalized.to_string(),
                            std::string::ToString::to_string,
                        )
                    };

                    ListRoutes {
                        uri,
                        actions: handler.actions.clone(),
                        method: handler.method.clone(),
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

    #[must_use]
    pub fn middlewares<H: Hooks>(&self, ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
        H::middlewares(ctx)
            .into_iter()
            .filter(|m| m.is_enabled())
            .collect::<Vec<Box<dyn MiddlewareLayer>>>()
    }

    /// Add the routes to an existing Axum Router, and set a list of middlewares
    /// that configure in the [`config::Config`]
    ///
    /// # Errors
    /// Return an [`Result`] when could not convert the router setup to
    /// [`axum::Router`].
    #[allow(clippy::cognitive_complexity)]
    pub fn to_router<H: Hooks>(
        &self,
        ctx: AppContext,
        mut app: AXRouter<AppContext>,
    ) -> Result<AXRouter> {
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
            if ctx
                .config
                .server
                .middlewares
                .cors
                .as_ref()
                .is_some_and(super::middleware::MiddlewareLayer::is_enabled)
            {
                app = app.layer(
                    tower::ServiceBuilder::new()
                        .layer(
                            ctx.config
                                .server
                                .middlewares
                                .cors
                                .clone()
                                .unwrap_or_default()
                                .cors()?,
                        )
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

        let middlewares = self.middlewares::<H>(&ctx);
        for mid in middlewares {
            app = mid.apply(app)?;
            tracing::info!(name = mid.name(), "+middleware");
        }
        let router = app.with_state(ctx);
        Ok(router)
    }
}

#[cfg(test)]
mod tests {

    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use tower::ServiceExt;

    use super::*;
    use crate::{prelude::*, tests_cfg};

    async fn action() -> Result<Response> {
        format::json("loco")
    }

    #[test]
    fn can_load_app_route_from_default() {
        for route in AppRoutes::with_default_routes().collect() {
            assert_debug_snapshot!(
                format!("[{}]", route.uri.replace('/', "[slash]")),
                format!("{:?} {}", route.actions, route.uri)
            );
        }
    }

    #[test]
    fn can_load_empty_app_routes() {
        assert_eq!(AppRoutes::empty().collect().len(), 0);
    }

    #[test]
    fn can_load_routes() {
        let router_without_prefix = Routes::new().add("/", get(action));
        let normalizer = Routes::new()
            .prefix("/normalizer")
            .add("no-slash", get(action))
            .add("/", post(action))
            .add("//loco///rs//", delete(action))
            .add("//////multiple-start", head(action))
            .add("multiple-end/////", trace(action));

        let app_router = AppRoutes::empty()
            .add_route(router_without_prefix)
            .add_route(normalizer)
            .add_routes(vec![
                Routes::new().add("multiple1", put(action)),
                Routes::new().add("multiple2", options(action)),
                Routes::new().add("multiple3", patch(action)),
            ]);

        for route in app_router.collect() {
            assert_debug_snapshot!(
                format!("[{}]", route.uri.replace('/', "[slash]")),
                format!("{:?} {}", route.actions, route.uri)
            );
        }
    }

    #[test]
    fn can_load_routes_with_root_prefix() {
        let router_without_prefix = Routes::new()
            .add("/loco", get(action))
            .add("loco-rs", get(action));

        let app_router = AppRoutes::empty()
            .prefix("api")
            .add_route(router_without_prefix);

        for route in app_router.collect() {
            assert_debug_snapshot!(
                format!("[{}]", route.uri.replace('/', "[slash]")),
                format!("{:?} {}", route.actions, route.uri)
            );
        }
    }

    #[rstest]
    #[case(axum::http::Method::GET, get(action))]
    #[case(axum::http::Method::POST, post(action))]
    #[case(axum::http::Method::DELETE, delete(action))]
    #[case(axum::http::Method::HEAD, head(action))]
    #[case(axum::http::Method::OPTIONS, options(action))]
    #[case(axum::http::Method::PATCH, patch(action))]
    #[case(axum::http::Method::POST, post(action))]
    #[case(axum::http::Method::PUT, put(action))]
    #[case(axum::http::Method::TRACE, trace(action))]
    #[tokio::test]
    async fn can_request_method(
        #[case] http_method: axum::http::Method,
        #[case] method: axum::routing::MethodRouter<AppContext>,
    ) {
        let router_without_prefix = Routes::new().add("/loco", method);

        let app_router = AppRoutes::empty().add_route(router_without_prefix);

        let ctx = tests_cfg::app::get_app_context().await;
        let router = app_router
            .to_router::<tests_cfg::db::AppHook>(ctx, axum::Router::new())
            .unwrap();

        let req = axum::http::Request::builder()
            .uri("/loco")
            .method(http_method)
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.oneshot(req).await.unwrap();
        assert!(response.status().is_success());
    }
}
