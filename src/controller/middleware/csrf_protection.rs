use super::MiddlewareLayer;
use crate::app::AppContext;
use crate::Result;
use axum::Router as AXRouter;
use axum_csrf::{CsrfConfig, CsrfLayer};
use serde::{Deserialize, Serialize};
use time::Duration as TimeDuration;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CsrfProtection {
    pub (crate) enable: Option<bool>,
    pub (crate) cookie: Option<CsrfCookie>,
    pub (crate) secure: Option<bool>,
    pub (crate) salt: Option<String>,
    pub (crate) prefix_with_host: Option<bool>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CsrfCookie {
    pub (crate) name: Option<String>,
    pub (crate) domain: Option<String>,
    pub (crate) path: Option<String>,
    pub (crate) lifetime: Option<i64>,
    pub (crate) http_only: Option<bool>,
    pub (crate) token_length: Option<usize>,
}

impl MiddlewareLayer for CsrfProtection {

    /// Returns the name of the middleware.
    fn name(&self) -> &'static str {
        "csrf_protection"
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Checks if the CSRF protection middleware is enabled.
    fn is_enabled(&self) -> bool {
        if let Some(enable) = &self.enable {
            if *enable {
                return true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }
    
    /// Applies the CSRF protection middleware.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {

        if let Some(true) = self.enable {

        let mut csrf_config = CsrfConfig::default();

        if let Some(cookie) = &self.cookie {

            if let Some(name) = &cookie.name {
                csrf_config = csrf_config.with_cookie_name(&name.clone());
            }
            if let Some(domain) = &cookie.domain {
                csrf_config = csrf_config.with_cookie_domain(Some(domain.clone()));
            }
            if let Some(path) = &cookie.path {
                csrf_config = csrf_config.with_cookie_path(path.clone());
            }
            if let Some(lifetime) = cookie.lifetime {
                csrf_config = csrf_config.with_lifetime(TimeDuration::seconds(lifetime));
            }
            if let Some(http_only) = cookie.http_only {
                csrf_config = csrf_config.with_http_only(http_only);
            }
            if let Some(token_length) = cookie.token_length {
                csrf_config = csrf_config.with_cookie_len(token_length);
            }
        }

        if let Some(secure) = self.secure {
            csrf_config = csrf_config.with_secure(secure);
        }
        if let Some(salt) = &self.salt {
            csrf_config = csrf_config.with_salt(salt.clone());
        }
        if let Some(prefix_with_host) = self.prefix_with_host {
            csrf_config = csrf_config.with_prefix_with_host(prefix_with_host);
        }

        let csrf_layer = CsrfLayer::new(csrf_config);
        let app = app.layer(csrf_layer);

        return Ok(app);

    } else {
        return Ok(app)
    }
}

}