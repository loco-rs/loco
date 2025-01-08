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
    request_context::{layer::RequestContextLayer, RequestContextError, TowerSessionStore},
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
            same_site: SameSite::Strict,
            expiry: Some(3600),
            secure: true,
            path: "/".to_string(),
            domain: None,
        }
    }
}

impl Default for RequestContextSession {
    fn default() -> Self {
        // If the session secret key is not configured in the environment config
        // file, generate a private key for the cookie session and panic.
        let private_key = Key::generate().master().to_vec();
        panic!(
            "Session secret key must be explicitly configured in your environment config file:
             request_context:
               session_store:
                 type: Cookie
                 value:
                   private_key: {private_key:?}
             "
        )
    }
}

impl Default for SameSite {
    fn default() -> Self {
        Self::Lax
    }
}

pub struct RequestContextMiddleware {
    config: RequestContextMiddlewareConfig,
    store: Option<TowerSessionStore>,
}

impl RequestContextMiddleware {
    #[must_use]
    pub fn new(config: RequestContextMiddlewareConfig, store: Option<TowerSessionStore>) -> Self {
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
        self.config.enable
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
                if private_key.len() < 64 {
                    return Err(RequestContextError::ConfigurationError(
                        "Session private key must be at least 64 bytes long".into(),
                    )
                    .into());
                }
                let layer = Self::get_cookie_request_context_middleware(
                    private_key,
                    &self.config.session_store,
                    &self.config.session_config,
                )?;
                app = app.layer(layer);
            }
            RequestContextSession::Tower => match self.store.as_ref() {
                Some(session_store) => {
                    let layer = Self::get_tower_request_context_middleware(
                        &self.config.session_store,
                        &self.config.session_config,
                    );
                    app = app.layer(layer);
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
        mut layer: SessionManagerLayer<TowerSessionStore>,
        config: &SessionCookieConfig,
    ) -> SessionManagerLayer<TowerSessionStore> {
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
    fn get_tower_request_context_middleware(
        session_config: &RequestContextSession,
        session_cookie_config: &SessionCookieConfig,
    ) -> RequestContextLayer {
        let key = Key::generate(); // Random generated since it is not used
        let store = crate::request_context::RequestContextStore::new(
            key,
            session_config.clone(),
            session_cookie_config.clone(),
        );
        RequestContextLayer::new(store)
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        response::{IntoResponse, Response},
        routing::{get, post},
        Extension, Router,
    };
    use tower::ServiceExt;

    use super::*;
    use crate::{controller::middleware::request_id, request_context::RequestContext, tests_cfg};
    const REQUEST_CONTEXT_DATA_KEY: &str = "alan";
    pub async fn create_request_context(mut req: RequestContext) -> Result<Response> {
        let data = "turing".to_string();
        req.insert(REQUEST_CONTEXT_DATA_KEY, data.clone()).await?;

        Ok(data.into_response())
    }

    pub async fn get_request_context(req: Extension<RequestContext>) -> Result<Response> {
        let data = req
            .get::<String>(REQUEST_CONTEXT_DATA_KEY)
            .await?
            .unwrap_or_default();
        println!("data: {:?}", data);

        Ok(data.into_response())
    }

    #[tokio::test]
    async fn test_request_context_middleware() {
        let middleware_config = RequestContextMiddlewareConfig {
            enable: true,
            session_config: SessionCookieConfig {
                name: "test_session".to_string(),
                http_only: true,
                secure: false, // For testing
                same_site: SameSite::Lax,
                expiry: Some(3600),
                path: "/".to_string(),
                domain: None,
            },
            session_store: RequestContextSession::Cookie {
                private_key: vec![
                    219, 25, 129, 200, 66, 52, 72, 66, 249, 60, 206, 40, 77, 150, 2, 8, 30, 192,
                    221, 5, 243, 74, 17, 172, 109, 96, 218, 46, 235, 118, 131, 150, 224, 205, 55,
                    147, 45, 151, 245, 23, 250, 48, 133, 115, 105, 252, 193, 15, 162, 167, 77, 189,
                    169, 91, 205, 172, 120, 254, 136, 111, 167, 161, 255, 107,
                ],
            },
        };
        // Need to apply LocoRequestId middleware before RequestContextMiddleware
        let request_id_middleware = request_id::RequestId { enable: true };
        // RequestContextMiddleware must be applied after LocoRequestId middleware
        let request_context_middleware = RequestContextMiddleware::new(middleware_config, None);
        let app = Router::new()
            .route("/request_context", post(create_request_context))
            .route("/request_context", get(get_request_context));

        let app = request_context_middleware
            .apply(app)
            .expect("apply middleware")
            .with_state(tests_cfg::app::get_app_context().await);
        let app = request_id_middleware
            .apply(app)
            .expect("apply request_id middleware")
            .with_state(tests_cfg::app::get_app_context().await);

        let req = Request::builder()
            .uri("/request_context")
            .method(Method::POST)
            .body(Body::empty())
            .expect("request");

        let response = app.clone().oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        // Verify session cookie is set
        let cookie_header = response
            .headers()
            .get("set-cookie")
            .expect("cookie header should be present");

        let cookie_str = cookie_header.to_str().expect("valid cookie string");
        assert!(cookie_str.contains("test_session="));
        assert!(cookie_str.contains("HttpOnly"));
        assert!(cookie_str.contains("Path=/"));
        assert!(cookie_str.contains("SameSite=Lax"));

        // Verify session cookie is retrieved
        let req = Request::builder()
            .uri("/request_context")
            .header("Cookie", cookie_str)
            .method(Method::GET)
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(req).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let response_body = response.into_body();
        let bytes = axum::body::to_bytes(response_body, usize::MAX)
            .await
            .unwrap();
        assert_eq!(bytes, "turing");
    }

    #[test]
    fn test_middleware_disabled() {
        let middleware = RequestContextMiddlewareConfig {
            enable: false,
            session_config: SessionCookieConfig::default(),
            session_store: RequestContextSession::Cookie {
                private_key: vec![0; 64],
            },
        };
        let middleware = RequestContextMiddleware::new(middleware, None);
        assert!(!middleware.is_enabled());
    }
}
