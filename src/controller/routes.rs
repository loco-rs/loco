use std::{convert::Infallible, fmt};

use axum::{extract::Request, response::IntoResponse, routing::Route};
use tower::{Layer, Service};
#[cfg(feature = "openapi")]
use utoipa_axum::router::{UtoipaMethodRouter, UtoipaMethodRouterExt};

use super::describe;
use crate::app::AppContext;

#[derive(Clone, Default, Debug)]
pub struct Routes {
    pub prefix: Option<String>,
    pub handlers: Vec<Handler>,
    // pub version: Option<String>,
}

#[derive(Clone)]
pub enum LocoMethodRouter {
    Axum(axum::routing::MethodRouter<AppContext>),
    #[cfg(feature = "openapi")]
    Utoipa(UtoipaMethodRouter),
}

#[derive(Clone, Debug)]
pub struct Handler {
    pub uri: String,
    pub method: LocoMethodRouter,
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
    pub fn add(mut self, uri: &str, method: impl Into<LocoMethodRouter>) -> Self {
        let method = method.into();
        let actions = match &method {
            LocoMethodRouter::Axum(m) => describe::method_action(m),
            #[cfg(feature = "openapi")]
            LocoMethodRouter::Utoipa(m) => describe::method_action(&m.2),
        };

        self.handlers.push(Handler {
            uri: uri.to_owned(),
            actions,
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
        L: Layer<Route> + Clone + Send + 'static,
        L::Service: Service<Request> + Clone + Send + 'static,
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
}

impl fmt::Debug for LocoMethodRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Axum(router) => write!(f, "{:?}", router),
            #[cfg(feature = "openapi")]
            Self::Utoipa(router) => {
                // Get the axum::routing::MethodRouter from the UtoipaMethodRouter wrapper
                write!(f, "{:?}", router.2)
            }
        }
    }
}

impl LocoMethodRouter {
    pub fn layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + 'static,
        L::Service: Service<Request> + Clone + Send + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        match self {
            LocoMethodRouter::Axum(router) => LocoMethodRouter::Axum(router.layer(layer)),
            #[cfg(feature = "openapi")]
            LocoMethodRouter::Utoipa(router) => LocoMethodRouter::Utoipa(router.layer(layer)),
        }
    }
}

impl From<axum::routing::MethodRouter<AppContext>> for LocoMethodRouter {
    fn from(router: axum::routing::MethodRouter<AppContext>) -> Self {
        LocoMethodRouter::Axum(router)
    }
}

#[cfg(feature = "openapi")]
impl From<UtoipaMethodRouter> for LocoMethodRouter {
    fn from(router: UtoipaMethodRouter) -> Self {
        LocoMethodRouter::Utoipa(router)
    }
}
