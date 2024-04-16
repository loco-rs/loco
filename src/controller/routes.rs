use std::convert::Infallible;

use axum::{extract::Request, response::IntoResponse, routing::Route};
use tower::{Layer, Service};

use super::describe;
use crate::app::AppContext;
#[derive(Clone, Default)]
pub struct Routes {
    pub prefix: Option<String>,
    pub handlers: Vec<Handler>,
    // pub version: Option<String>,
}

#[derive(Clone, Default)]
pub struct Handler {
    pub uri: String,
    pub method: axum::routing::MethodRouter<AppContext>,
    pub actions: Vec<axum::http::Method>,
}

impl Handler {
    pub fn layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + 'static,
        L::Service: Service<Request> + Clone + Send + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        Self {
            uri: self.uri,
            actions: self.actions,
            method: self.method.layer(layer),
        }
    }
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
