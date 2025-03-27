//! This module defines the [`AppRoutes`] struct that is responsible for
//! configuring routes in an Axum application. It allows you to define route
//! prefixes, add routes, and configure middlewares for the application.

use std::{fmt, sync::OnceLock};

use axum::Router as AXRouter;
use regex::Regex;
#[cfg(any(
    feature = "openapi_swagger",
    feature = "openapi_redoc",
    feature = "openapi_scalar"
))]
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
#[cfg(feature = "openapi_redoc")]
use utoipa_redoc::{Redoc, Servable};
#[cfg(feature = "openapi_scalar")]
use utoipa_scalar::{Scalar, Servable as ScalarServable};
#[cfg(feature = "openapi_swagger")]
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    app::{AppContext, Hooks},
    controller::{
        middleware::MiddlewareLayer,
        routes::{LocoMethodRouter, Routes},
    },
    Result,
};
#[cfg(any(
    feature = "openapi_swagger",
    feature = "openapi_redoc",
    feature = "openapi_scalar"
))]
use crate::{config::OpenAPIType, controller::openapi};

static NORMALIZE_URL: OnceLock<Regex> = OnceLock::new();

fn get_normalize_url() -> &'static Regex {
    NORMALIZE_URL.get_or_init(|| Regex::new(r"/+").unwrap())
}

/// Represents the routes of the application.
#[derive(Clone)]
pub struct AppRoutes {
    prefix: Option<String>,
    routes: Vec<Routes>,
}

#[derive(Debug)]
pub struct ListRoutes {
    pub uri: String,
    pub actions: Vec<axum::http::Method>,
    pub method: LocoMethodRouter,
}

impl fmt::Display for ListRoutes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let actions_str = self
            .actions
            .iter()
            .map(ToString::to_string)
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
        }
    }

    #[must_use]
    pub fn collect(&self) -> Vec<ListRoutes> {
        self.get_routes()
            .iter()
            .flat_map(|controller| {
                let uri_parts = controller
                    .prefix
                    .as_ref()
                    .map_or_else(Vec::new, |prefix| vec![prefix.to_string()]);

                controller.handlers.iter().map(move |handler| {
                    let mut parts = uri_parts.clone();
                    parts.push(handler.uri.to_string());
                    let joined_parts = parts.join("/");

                    let normalized = get_normalize_url().replace_all(&joined_parts, "/");
                    let mut uri = if normalized == "/" {
                        normalized.to_string()
                    } else {
                        normalized
                            .strip_suffix('/')
                            .map_or_else(|| normalized.to_string(), ToString::to_string)
                    };

                    if !uri.starts_with('/') {
                        uri.insert(0, '/');
                    }

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
        let mut prefix = prefix.to_owned();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        if !prefix.starts_with('/') {
            prefix.insert(0, '/');
        }

        self.prefix = Some(prefix);

        self
    }

    /// Set a nested prefix for the routes. This prefix will be appended to any existing prefix.
    ///
    /// # Example
    ///
    /// In the following example, you are adding `api` as a prefix and then nesting `v1` within it:
    ///
    /// ```rust
    /// use loco_rs::controller::AppRoutes;
    /// use loco_rs::tests_cfg::*;
    ///
    /// let app_routes = AppRoutes::with_default_routes()
    ///      .prefix("api")
    ///      .add_route(controllers::auth::routes())
    ///      .nest_prefix("v1")
    ///      .add_route(controllers::home::routes());
    ///
    /// // This will result in routes like `/api/auth` and `/api/v1/home`
    /// ```
    #[must_use]
    pub fn nest_prefix(mut self, prefix: &str) -> Self {
        let prefix = self.prefix.as_ref().map_or_else(
            || prefix.to_owned(),
            |old_prefix| format!("{old_prefix}{prefix}"),
        );
        self = self.prefix(&prefix);

        self
    }

    /// Set a nested route with a prefix. This route will be added with the specified prefix.
    /// The prefix will only be applied to the routes given in this function.
    ///
    /// # Example
    ///
    /// In the following example, you are adding `api` as a prefix and then nesting a route within it:
    ///
    /// ```rust
    /// use axum::routing::get;
    /// use loco_rs::controller::{AppRoutes, Routes};
    ///
    /// let route = Routes::new().add("/notes", get(|| async { "notes" }));
    /// let app_routes = AppRoutes::with_default_routes()
    ///     .prefix("api")
    ///     .nest_route("v1", route);
    ///
    /// // This will result in routes with the prefix `/api/v1/notes`
    /// ```
    #[must_use]
    pub fn nest_route(mut self, prefix: &str, route: Routes) -> Self {
        let old_prefix = self.prefix.clone();
        self = self.nest_prefix(prefix);
        self = self.add_route(route);
        self.prefix = old_prefix;

        self
    }

    /// Set multiple nested routes with a prefix. These routes will be added with the specified prefix.
    /// The prefix will only be applied to the routes given in this function.
    ///
    /// # Example
    ///
    /// In the following example, you are adding `api` as a prefix and then nesting multiple routes within it:
    ///
    /// ```rust
    /// use axum::routing::get;
    /// use loco_rs::controller::{AppRoutes, Routes};
    ///
    /// let routes = vec![
    ///     Routes::new().add("/notes", get(|| async { "notes" })),
    ///     Routes::new().add("/users", get(|| async { "users" })),
    /// ];
    /// let app_routes = AppRoutes::with_default_routes()
    ///     .prefix("api")
    ///     .nest_routes("v1", routes);
    ///
    /// // This will result in routes with the prefix `/api/v1/notes` and `/api/v1/users`
    /// ```
    #[must_use]
    pub fn nest_routes(mut self, prefix: &str, routes: Vec<Routes>) -> Self {
        let old_prefix = self.prefix.clone();
        self = self.nest_prefix(prefix);
        self = self.add_routes(routes);
        self.prefix = old_prefix;

        self
    }

    /// Add a single route.
    #[must_use]
    pub fn add_route(mut self, mut route: Routes) -> Self {
        let routes_prefix = {
            if let Some(mut prefix) = self.prefix.clone() {
                let routes_prefix = route.prefix.clone().unwrap_or_default();

                prefix.push_str(routes_prefix.as_str());
                Some(prefix)
            } else {
                route.prefix.clone()
            }
        };

        if let Some(prefix) = routes_prefix {
            route = route.prefix(prefix.as_str());
        }

        self.routes.push(route);

        self
    }

    /// Add multiple routes.
    #[must_use]
    pub fn add_routes(mut self, mounts: Vec<Routes>) -> Self {
        for mount in mounts {
            self = self.add_route(mount);
        }

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
        #[cfg(any(
            feature = "openapi_swagger",
            feature = "openapi_redoc",
            feature = "openapi_scalar"
        ))]
        let mut api_router: OpenApiRouter<AppContext> =
            OpenApiRouter::with_openapi(H::inital_openapi_spec(&ctx));

        for router in self.collect() {
            tracing::info!("{}", router.to_string());
            match router.method {
                LocoMethodRouter::Axum(method) => {
                    app = app.route(&router.uri, method);
                }
                #[cfg(any(
                    feature = "openapi_swagger",
                    feature = "openapi_redoc",
                    feature = "openapi_scalar"
                ))]
                LocoMethodRouter::Utoipa(method) => {
                    app = app.route(&router.uri, method.2.clone());
                    api_router = api_router.routes(method.with_state(ctx.clone()));
                }
            }
        }

        #[cfg(any(
            feature = "openapi_swagger",
            feature = "openapi_redoc",
            feature = "openapi_scalar"
        ))]
        {
            // Collect the OpenAPI spec
            let (_, open_api_spec) = api_router.split_for_parts();
            openapi::set_openapi_spec(open_api_spec);
        }

        // Serve the OpenAPI spec using the enabled OpenAPI visualizers
        #[cfg(feature = "openapi_redoc")]
        {
            if let Some(OpenAPIType::Redoc {
                url,
                spec_json_url,
                spec_yaml_url,
            }) = ctx.config.server.openapi.redoc.clone()
            {
                app = app.merge(Redoc::with_url(url, openapi::get_openapi_spec().clone()));
                app = openapi::add_openapi_endpoints(app, spec_json_url, spec_yaml_url);
            }
        }

        #[cfg(feature = "openapi_scalar")]
        {
            if let Some(OpenAPIType::Scalar {
                url,
                spec_json_url,
                spec_yaml_url,
            }) = ctx.config.server.openapi.scalar.clone()
            {
                app = app.merge(Scalar::with_url(url, openapi::get_openapi_spec().clone()));
                app = openapi::add_openapi_endpoints(app, spec_json_url, spec_yaml_url);
            }
        }

        #[cfg(feature = "openapi_swagger")]
        {
            if let Some(OpenAPIType::Swagger {
                url,
                spec_json_url,
                spec_yaml_url,
            }) = ctx.config.server.openapi.swagger.clone()
            {
                app = app.merge(
                    SwaggerUi::new(url).url(spec_json_url, openapi::get_openapi_spec().clone()),
                );
                app = openapi::add_openapi_endpoints(app, None, spec_yaml_url);
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
    use super::*;
    use crate::{prelude::*, tests_cfg};
    use axum::http::Method;
    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use std::vec;
    use tower::ServiceExt;

    async fn action() -> Result<Response> {
        format::json("loco")
    }

    #[test]
    fn can_load_app_route_from_default() {
        let routes = AppRoutes::with_default_routes().collect();

        for route in routes {
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

    #[test]
    fn can_nest_prefix() {
        let app_router = AppRoutes::empty().prefix("api").nest_prefix("v1");

        assert_eq!(app_router.get_prefix().unwrap(), "/api/v1/");
    }

    #[test]
    fn can_nest_route() {
        let route = Routes::new().add("/notes", get(action));
        let app_router = AppRoutes::empty().prefix("api").nest_route("v1", route);

        let routes = app_router.collect();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].uri, "/api/v1/notes");
    }

    #[test]
    fn can_nest_routes() {
        let routes = vec![
            Routes::new().add("/notes", get(action)),
            Routes::new().add("/users", get(action)),
        ];
        let app_router = AppRoutes::empty().prefix("api").nest_routes("v1", routes);

        for route in app_router.collect() {
            assert_debug_snapshot!(
                format!("[{}]", route.uri.replace('/', "[slash]")),
                format!("{:?} {}", route.actions, route.uri)
            );
        }
    }

    #[rstest]
    #[case(Method::GET, get(action))]
    #[case(Method::POST, post(action))]
    #[case(Method::DELETE, delete(action))]
    #[case(Method::HEAD, head(action))]
    #[case(Method::OPTIONS, options(action))]
    #[case(Method::PATCH, patch(action))]
    #[case(Method::POST, post(action))]
    #[case(Method::PUT, put(action))]
    #[case(Method::TRACE, trace(action))]
    #[tokio::test]
    async fn can_request_method(
        #[case] http_method: Method,
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
