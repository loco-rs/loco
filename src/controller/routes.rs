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
    /// 
    /// use rustyrails::{
    ///     controller::{Routes, format},
    ///     Result,
    /// };
    /// use axum::{routing::get, Json};
    /// use serde::Serialize;;
    ///
    /// #[derive(Serialize)]
    /// struct Health {
    ///    pub ok: bool,
    /// }
    ///
    /// async fn ping() -> Result<Json<Health>> {
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
    /// 
    /// use rustyrails::{
    ///     Result,
    ///     controller::{Routes, format},
    /// };
    /// use axum::{routing::get, Json};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Health {
    ///    pub ok: bool,
    /// }
    ///
    /// async fn ping() -> Result<Json<Health>> {
    ///     format::json(Health { ok: true })
    /// }
    /// Routes::new().add("/_ping", get(ping));
    ///    
    /// ````
    #[must_use]
    pub fn add(mut self, uri: &str, method: axum::routing::MethodRouter<AppContext>) -> Self {
        self.handlers.push(Handler {
            uri: uri.to_owned(),
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
    /// 
    /// use rustyrails::{
    ///     controller::{Routes, format},
    ///     Result,
    /// };
    /// use axum::{routing::get, Json};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Health {
    ///    pub ok: bool,
    /// }
    ///
    /// async fn ping() -> Result<Json<Health>> {
    ///     format::json(Health { ok: true })
    /// }
    /// Routes::new().prefix("status").add("/_ping", get(ping));
    ///    
    /// ````
    #[must_use]
    pub fn prefix(mut self, uri: &str) -> Self {
        self.prefix = Some(uri.to_owned());
        self
    }
}
