use axum::Router as AXRouter;
use serde::{Deserialize, Serialize};
use tower_sessions::{
    cookie,
    cookie::{time, Key},
    Expiry, SessionManagerLayer,
};

use crate::{
    app::AppContext,
    controller::middleware::MiddlewareLayer,
    request_context::{layer::RequestContextLayer, CustomSessionStore},
    Result,
};

/// Request context configuration
/// # Example:
/// ```yaml
/// # config/development.yaml
/// request_context:
///  enable: true
///  session_config:
///    name: session
///    http_only: true
///    same_site:
///      type: Lax
///    expiry: 3600
///    secure: false
///    path: /
///  #  domain: ""
///  session_store:
///    type: Cookie
///    value:
///     private_key: <your private key>
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestContextMiddlewareConfig {
    pub enable: bool,
    pub session_config: SessionCookieConfig,
    pub session_store: RequestContextSession,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SessionCookieConfig {
    pub name: String,
    pub http_only: bool,
    pub same_site: SameSite,
    pub expiry: Option<i32>, // in seconds
    pub secure: bool,
    pub path: String,
    pub domain: Option<String>,
}

/// `RequestContextSession` configuration
/// # Enums:
/// * Cookie - this is a placeholder for when we implement the cookie session
///   driver or our custom session.
/// * Tower - this is a placeholder for when we implement the tower session
///   driver or our custom session.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum RequestContextSession {
    /// Cookie session configuration
    Cookie {
        /// Private key for Private Cookie Jar in Cookie Sessions, must be more
        /// than 64 bytes.
        private_key: Vec<u8>,
    },
    /// Tower session configuration
    Tower,
}
/// `SameSite` cookie configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum SameSite {
    Lax,
    Strict,
    None,
}

impl Default for RequestContextMiddlewareConfig {
    fn default() -> Self {
        Self {
            enable: true,
            session_config: SessionCookieConfig::default(),
            session_store: RequestContextSession::default(),
        }
    }
}

impl Default for SessionCookieConfig {
    fn default() -> Self {
        Self {
            name: " __loco_session".to_string(),
            http_only: true,
            same_site: SameSite::default(),
            expiry: None,
            secure: false,
            path: "/".to_string(),
            domain: None,
        }
    }
}

impl Default for RequestContextSession {
    fn default() -> Self {
        // Generate a private key for the cookie session
        let private_key = Key::generate().master().to_vec();
        tracing::info!(
            "[Middleware] Generating private key for cookie session: {:?}",
            private_key
        );
        Self::Cookie { private_key }
    }
}

impl Default for SameSite {
    fn default() -> Self {
        Self::Lax
    }
}

pub struct RequestContextMiddleware {
    config: RequestContextMiddlewareConfig,
    store: Option<CustomSessionStore>,
}

impl RequestContextMiddleware {
    pub fn new(config: RequestContextMiddlewareConfig, store: Option<CustomSessionStore>) -> Self {
        Self { config, store }
    }
}

impl MiddlewareLayer for RequestContextMiddleware {
    /// Returns the name of the middleware.
    fn name(&self) -> &'static str {
        "RequestContextMiddleware"
    }

    /// Returns whether the middleware is enabled or not.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Returns middleware config.
    ///
    /// # Errors
    /// when could not convert middleware to [`serde_json::Value`]
    fn config(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::json!(self.config))
    }

    /// Applies the middleware to the given Axum router and returns the modified
    /// router.
    ///
    /// # Errors
    ///
    /// If there is an issue when adding the middleware to the router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        self.add_request_context_middleware(app)
    }
}

impl RequestContextMiddleware {
    fn add_request_context_middleware(
        &self,
        mut app: AXRouter<AppContext>,
    ) -> Result<AXRouter<AppContext>> {
        // Add the request context middleware
        match &self.config.session_store {
            RequestContextSession::Cookie { private_key } => {
                tracing::info!("[Middleware] Adding request context");
                let layer = Self::get_cookie_request_context_middleware(
                    private_key,
                    &self.config.session_store,
                    &self.config.session_config,
                )?;
                app = app.layer(layer);
            }
            RequestContextSession::Tower => match self.store.as_ref() {
                Some(session_store) => {
                    tracing::info!("[Middleware] Adding request context");
                    let layer = SessionManagerLayer::new(session_store.to_owned());
                    let layer =
                        Self::add_request_context_config_tower(layer, &self.config.session_config);
                    app = app.layer(layer);
                }
                None => {
                    tracing::error!("request context session store not configured");
                }
            },
        }
        Ok(app)
    }

    fn add_request_context_config_tower(
        mut layer: SessionManagerLayer<CustomSessionStore>,
        config: &SessionCookieConfig,
    ) -> SessionManagerLayer<CustomSessionStore> {
        layer = layer.with_name(config.name.to_string());
        if config.http_only {
            layer = layer.with_http_only(true);
        }
        if config.secure {
            layer = layer.with_secure(true);
        }
        match config.same_site {
            SameSite::Strict => layer = layer.with_same_site(cookie::SameSite::Strict),
            SameSite::Lax => layer = layer.with_same_site(cookie::SameSite::Lax),
            SameSite::None => layer = layer.with_same_site(cookie::SameSite::None),
        }
        if let Some(expiry) = &config.expiry {
            tracing::info!("request context session expiry: {:?}", expiry);
            let expiry = Expiry::OnInactivity(time::Duration::seconds(i64::from(*expiry)));
            layer = layer.with_expiry(expiry);
        }
        layer
    }

    fn get_cookie_request_context_middleware(
        private_key: &[u8],
        session_config: &RequestContextSession,
        session_cookie_config: &SessionCookieConfig,
    ) -> Result<RequestContextLayer> {
        let private_key = Key::try_from(private_key).map_err(|e| {
            tracing::error!(error = ?e, "could not convert private key from configuration");
            crate::prelude::Error::Message(
                "could not convert private key from configuration".to_string(),
            )
        })?;
        let store = crate::request_context::RequestContextStore::new(
            private_key,
            session_config.clone(),
            session_cookie_config.clone(),
        );
        Ok(RequestContextLayer::new(store))
    }
}
