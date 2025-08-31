use std::convert::Infallible;

use axum::{extract::Request, response::IntoResponse, routing::Route};
use tower::{Layer, Service};

use super::describe;
use crate::app::AppContext;
#[derive(Clone, Default, Debug)]
pub struct Routes {
    pub prefix: Option<String>,
    pub handlers: Vec<Handler>,
    // pub version: Option<String>,
}

#[derive(Clone, Default, Debug)]
pub struct Handler {
    pub uri: String,
    pub method: axum::routing::MethodRouter<AppContext>,
    pub actions: Vec<axum::http::Method>,
}

impl Routes {
    /// Creates a new [`Routes`] instance with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a prefix for the routes. this prefix will be a prefix for all the
    /// routes.
    ///
    /// # Example
    ///
    /// In the following example the we are adding `status`  as a prefix to the
    /// _ping endpoint HOST/status/_ping.
    ///
    /// ```rust
    /// use loco_rs::prelude::*;
    /// use serde::Serialize;;
    ///
    /// #[derive(Serialize)]
    /// struct Health {
    ///    pub ok: bool,
    /// }
    ///
    /// async fn ping() -> Result<Response> {
    ///     format::json(Health { ok: true })
    /// }
    /// Routes::at("status").add("/_ping", get(ping));
    ///    
    /// ````
    #[must_use]
    pub fn at(prefix: &str) -> Self {
        Self {
            prefix: Some(prefix.to_string()),
            ..Self::default()
        }
    }

    /// Adding new router
    ///
    /// # Example
    ///
    /// This example preset how to add a get endpoint int the Router.
    ///
    /// ```rust
    /// use loco_rs::prelude::*;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Health {
    ///    pub ok: bool,
    /// }
    ///
    /// async fn ping() -> Result<Response> {
    ///     format::json(Health { ok: true })
    /// }
    /// Routes::new().add("/_ping", get(ping));
    /// ````
    #[must_use]
    pub fn add(mut self, uri: &str, method: axum::routing::MethodRouter<AppContext>) -> Self {
        describe::method_action(&method);
        self.handlers.push(Handler {
            uri: uri.to_owned(),
            actions: describe::method_action(&method),
            method,
        });
        self
    }

    /// Set a prefix for the routes. this prefix will be a prefix for all the
    /// routes.
    ///
    /// # Example
    ///
    /// In the following example the we are adding `status`  as a prefix to the
    /// _ping endpoint HOST/status/_ping.
    ///
    /// ```rust
    /// use loco_rs::prelude::*;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Health {
    ///    pub ok: bool,
    /// }
    ///
    /// async fn ping() -> Result<Response> {
    ///     format::json(Health { ok: true })
    /// }
    /// Routes::new().prefix("status").add("/_ping", get(ping));
    /// ````
    #[must_use]
    pub fn prefix(mut self, uri: &str) -> Self {
        self.prefix = Some(uri.to_owned());
        self
    }

    /// Set a layer for the routes. this layer will be a layer for all the
    /// routes.
    ///
    /// # Example
    ///
    /// In the following example, we are adding a layer to the routes.
    ///
    /// ```rust
    /// use loco_rs::prelude::*;
    /// use tower::{Layer, Service};
    /// use tower_http::timeout::TimeoutLayer;
    /// async fn ping() -> Result<Response> {
    ///     format::json("Ok")
    /// }
    /// Routes::new().prefix("status").add("/_ping", get(ping)).layer(TimeoutLayer::new(std::time::Duration::from_secs(5)));
    /// ```
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        Self {
            prefix: self.prefix,
            handlers: self
                .handlers
                .iter()
                .map(|handler| Handler {
                    uri: handler.uri.clone(),
                    actions: handler.actions.clone(),
                    method: handler.method.clone().layer(layer.clone()),
                })
                .collect(),
        }
    }

    /// Nest another Routes instance under a prefix path.
    ///
    /// This method allows you to nest a group of routes under a specific path prefix,
    /// similar to Axum's `nest` method. The nested routes will have their URIs
    /// prefixed with the given path.
    ///
    /// # Example
    ///
    /// ```rust
    /// use loco_rs::prelude::*;
    /// use axum::routing::{get, post, delete, patch};
    ///
    /// // Define user-related handlers
    /// async fn list_users() -> Result<Response> {
    ///     format::json("users list")
    /// }
    ///
    /// async fn get_user() -> Result<Response> {
    ///     format::json("user detail")
    /// }
    ///
    /// async fn create_user() -> Result<Response> {
    ///     format::json("user created")
    /// }
    ///
    /// async fn update_user() -> Result<Response> {
    ///     format::json("user updated")
    /// }
    ///
    /// async fn delete_user() -> Result<Response> {
    ///     format::json("user deleted")
    /// }
    ///
    /// // Create API routes for users
    /// let user_routes = Routes::new()
    ///     .add("/users", get(list_users))
    ///     .add("/users", post(create_user))
    ///     .add("/users/{id}", get(get_user))
    ///     .add("/users/{id}", patch(update_user))
    ///     .add("/users/{id}", delete(delete_user));
    ///
    /// // Create the main application routes
    /// let app_routes = Routes::new()
    ///     .add("/health", get(|| async { "ok" }))
    ///     .nest("/api/v1", user_routes);
    ///
    /// // This will result in routes:
    /// // - GET /health
    /// // - GET /api/v1/users
    /// // - POST /api/v1/users
    /// // - GET /api/v1/users/{id}
    /// // - PATCH /api/v1/users/{id}
    /// // - DELETE /api/v1/users/{id}
    /// ```
    #[must_use]
    pub fn nest(mut self, path: &str, nested_routes: Routes) -> Self {
        // Normalize the path to ensure it starts with / and doesn't end with /
        let mut normalized_path = path.to_string();
        if !normalized_path.starts_with('/') {
            normalized_path.insert(0, '/');
        }
        if normalized_path.ends_with('/') && normalized_path != "/" {
            normalized_path.pop();
        }

        // Process each handler from the nested routes
        for handler in nested_routes.handlers {
            // Combine the path prefix with the handler's URI
            let combined_uri = if handler.uri == "/" {
                normalized_path.clone()
            } else {
                format!("{}{}", normalized_path, handler.uri)
            };

            // Create a new handler with the combined URI
            let new_handler = Handler {
                uri: combined_uri,
                method: handler.method,
                actions: handler.actions,
            };

            self.handlers.push(new_handler);
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use axum::routing::get;

    async fn users() -> Result<Response> {
        format::json("users list")
    }

    async fn user_detail() -> Result<Response> {
        format::json("user detail")
    }

    async fn ping() -> Result<Response> {
        format::json("pong")
    }

    #[test]
    fn test_nest_method() {
        // Create nested routes
        let api_routes = Routes::new()
            .add("/users", get(users))
            .add("/users/{id}", get(user_detail));

        // Nest them under /api
        let app_routes = Routes::new()
            .add("/ping", get(ping))
            .nest("/api", api_routes);

        // Verify the routes are correctly nested
        assert_eq!(app_routes.handlers.len(), 3);

        // Check that the ping route is unchanged
        let ping_handler = &app_routes.handlers[0];
        assert_eq!(ping_handler.uri, "/ping");

        // Check that the nested routes have the correct prefixes
        let users_handler = &app_routes.handlers[1];
        assert_eq!(users_handler.uri, "/api/users");

        let user_detail_handler = &app_routes.handlers[2];
        assert_eq!(user_detail_handler.uri, "/api/users/{id}");
    }

    #[test]
    fn test_nest_method_with_root_path() {
        // Create nested routes with a root path
        let api_routes = Routes::new()
            .add("/", get(users))
            .add("/users", get(user_detail));

        // Nest them under /api
        let app_routes = Routes::new().nest("/api", api_routes);

        // Verify the routes are correctly nested
        assert_eq!(app_routes.handlers.len(), 2);

        // Check that the root path is handled correctly
        let root_handler = &app_routes.handlers[0];
        assert_eq!(root_handler.uri, "/api");

        let users_handler = &app_routes.handlers[1];
        assert_eq!(users_handler.uri, "/api/users");
    }

    #[test]
    fn test_nest_method_with_trailing_slash() {
        // Create nested routes
        let api_routes = Routes::new().add("/users", get(users));

        // Nest them under /api/ (with trailing slash)
        let app_routes = Routes::new().nest("/api/", api_routes);

        // Verify the routes are correctly nested (trailing slash should be removed)
        assert_eq!(app_routes.handlers.len(), 1);

        let users_handler = &app_routes.handlers[0];
        assert_eq!(users_handler.uri, "/api/users");
    }

    #[test]
    fn test_nest_method_without_leading_slash() {
        // Create nested routes
        let api_routes = Routes::new().add("/users", get(users));

        // Nest them under api (without leading slash)
        let app_routes = Routes::new().nest("api", api_routes);

        // Verify the routes are correctly nested (leading slash should be added)
        assert_eq!(app_routes.handlers.len(), 1);

        let users_handler = &app_routes.handlers[0];
        assert_eq!(users_handler.uri, "/api/users");
    }
}
