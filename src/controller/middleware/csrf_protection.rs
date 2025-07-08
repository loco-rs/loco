use super::MiddlewareLayer;
use crate::app::AppContext;
use crate::Result;
use axum::Router as AXRouter;
use axum_csrf::{CsrfConfig, CsrfLayer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CsrfProtection {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for CsrfProtection {

    fn name(&self) -> &'static str {
        "csrf_protection"
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        let csrf_config = CsrfConfig::default();
        let csrf_layer = CsrfLayer::new(csrf_config);
        let app = app.layer(csrf_layer);
        Ok(app)
    }

}